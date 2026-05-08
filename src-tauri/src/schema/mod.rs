//! Phase 2 schema resolution chain (D-24): SQLite cache → local Steam appcache
//! → Goldberg achievements.json metadata → public Steam Web API rarity →
//! cache-back. Plan 05's popup_queue calls `lookup()` synchronously at fire-time;
//! Plan 03's game_detect spawns `resolve()` on game-start (D-25 trigger).

pub mod cache;
pub mod appcache;
pub mod steam_api;
pub mod goldberg_meta;

use std::path::PathBuf;
use std::sync::Arc;
use serde::Serialize;
use crate::store::SqliteStore;

/// Public achievement schema as consumed by Plan 05 (popup) and Plan 06 (companion).
/// Mirrors `src/types.ts::AchievementSchema` field-for-field via serde.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AchievementSchema {
    pub app_id: u64,
    pub ach_api_name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub icon_path: Option<String>,
    pub hidden: bool,
    pub global_pct: Option<f64>,
}

impl AchievementSchema {
    fn from_row(row: cache::SchemaCacheRow) -> Self {
        Self {
            app_id: row.app_id,
            ach_api_name: row.ach_api_name,
            display_name: row.display_name,
            description: row.description,
            icon_path: row.icon_path,
            hidden: row.hidden,
            global_pct: row.global_pct,
        }
    }
}

/// Tier classification per CONTEXT.md D-27 + D-07.
/// rare iff Some(p) and p < 10.0; otherwise standard (None → graceful degrade).
/// Returns &'static str so callers can plug it directly into PopupPayload.tier.
pub fn classify_tier(global_pct: Option<f64>) -> &'static str {
    match global_pct {
        Some(p) if p < 10.0 => "rare",
        _ => "standard",
    }
}

/// Resolution-chain orchestrator. Cheap to clone (Arc internals).
#[derive(Clone)]
pub struct SchemaCache {
    store: Arc<SqliteStore>,
    http: reqwest::Client,
}

impl SchemaCache {
    pub fn new(store: Arc<SqliteStore>) -> anyhow::Result<Self> {
        // 8s timeout default; explicit User-Agent helps Valve's logs identify
        // legitimate clients (no API key, but courteous identification).
        let http = reqwest::Client::builder()
            .user_agent(concat!("Hallmark/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(8))
            .build()?;
        Ok(Self { store, http })
    }

    /// Synchronous-style cache read for popup-queue's hot path.
    /// Returns None if the row hasn't been resolved yet (popup falls back per D-26).
    pub fn lookup(&self, app_id: u64, ach_api_name: &str) -> Option<AchievementSchema> {
        match self.store.with_conn(|c| cache::get_schema_row(c, app_id, ach_api_name)) {
            Ok(Some(row)) => Some(AchievementSchema::from_row(row)),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(app_id, ach = ach_api_name, error = %e, "schema lookup failed");
                None
            }
        }
    }

    /// Read the full cached achievement list for one app.
    /// Used by Plan 06's companion to render "earned + locked" view.
    /// Empty Vec when no schema cached yet.
    pub fn list_for_app(&self, app_id: u64) -> Vec<AchievementSchema> {
        match self.store.with_conn(|c| cache::get_schema_for_app(c, app_id)) {
            Ok(rows) => rows.into_iter().map(AchievementSchema::from_row).collect(),
            Err(e) => {
                tracing::warn!(app_id, error = %e, "schema list_for_app failed");
                Vec::new()
            }
        }
    }

    /// Async resolution kicked off on game-start (D-25). Walks the D-24
    /// lookup chain: Goldberg metadata first (most reliable for emulated
    /// games), then public Web API rarity merge. Emits `schema-resolved`
    /// event via the AppHandle when each leg completes so Plan 06 can
    /// in-place upgrade the companion list.
    ///
    /// Each leg is independently fallible; failure of one leg logs at warn
    /// and continues to the next. We never propagate an error out of this
    /// function — partial cache is better than no cache.
    ///
    /// `goldberg_json_paths` is the list of Goldberg achievements.json files
    /// on disk for this app_id (Plan 03 supplies these from path discovery).
    /// Pass empty Vec for legitimate-Steam games (Phase 3 territory).
    pub async fn resolve(
        &self,
        app: tauri::AppHandle,
        app_id: u64,
        goldberg_json_paths: Vec<PathBuf>,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // ---- Leg 1: Goldberg metadata ----
        for path in &goldberg_json_paths {
            match read_with_retry(path).await {
                Ok(json) => {
                    match goldberg_meta::parse_goldberg_metadata(&json) {
                        Ok(metas) => {
                            tracing::info!(
                                app_id,
                                path = %path.display(),
                                count = metas.len(),
                                "Goldberg metadata parsed; merging into schema_cache"
                            );
                            for meta in metas {
                                // Read existing row (may have rarity from prior session) and merge.
                                let existing = self
                                    .store
                                    .with_conn(|c| cache::get_schema_row(c, app_id, &meta.api_name))
                                    .ok()
                                    .flatten();
                                let row = cache::SchemaCacheRow {
                                    app_id,
                                    ach_api_name: meta.api_name.clone(),
                                    display_name: meta.display_name.or_else(|| {
                                        existing.as_ref().and_then(|r| r.display_name.clone())
                                    }),
                                    description: meta.description.or_else(|| {
                                        existing.as_ref().and_then(|r| r.description.clone())
                                    }),
                                    icon_path: meta.icon_url.or_else(|| {
                                        existing.as_ref().and_then(|r| r.icon_path.clone())
                                    }),
                                    hidden: meta.hidden,
                                    global_pct: existing.as_ref().and_then(|r| r.global_pct),
                                    cached_at: now,
                                };
                                if let Err(e) =
                                    self.store.with_conn(|c| cache::upsert_schema(c, &row))
                                {
                                    tracing::warn!(
                                        app_id,
                                        ach = %row.ach_api_name,
                                        error = %e,
                                        "schema upsert failed"
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(app_id, error = %e, "Goldberg metadata parse failed")
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "Goldberg JSON read failed")
                }
            }
        }

        // Emit resolved event after Goldberg leg so companion can render skeleton + names.
        let _ = tauri::Emitter::emit(
            &app,
            "schema-resolved",
            serde_json::json!({"app_id": app_id, "stage": "metadata"}),
        );

        // ---- Leg 2: Public Steam Web API rarity ----
        match steam_api::fetch_global_pcts(&self.http, app_id).await {
            Ok(pcts) => {
                tracing::info!(app_id, count = pcts.len(), "global pcts fetched; merging");
                for (api_name, pct) in pcts {
                    let existing = self
                        .store
                        .with_conn(|c| cache::get_schema_row(c, app_id, &api_name))
                        .ok()
                        .flatten();
                    let row = cache::SchemaCacheRow {
                        app_id,
                        ach_api_name: api_name,
                        display_name: existing.as_ref().and_then(|r| r.display_name.clone()),
                        description: existing.as_ref().and_then(|r| r.description.clone()),
                        icon_path: existing.as_ref().and_then(|r| r.icon_path.clone()),
                        hidden: existing.as_ref().map(|r| r.hidden).unwrap_or(false),
                        global_pct: Some(pct),
                        cached_at: now,
                    };
                    if let Err(e) = self.store.with_conn(|c| cache::upsert_schema(c, &row)) {
                        tracing::warn!(app_id, error = %e, "rarity upsert failed");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    app_id,
                    error = %e,
                    "global pcts fetch failed; continuing without rarity"
                )
            }
        }

        // Emit final resolved event after rarity leg.
        let _ = tauri::Emitter::emit(
            &app,
            "schema-resolved",
            serde_json::json!({"app_id": app_id, "stage": "rarity"}),
        );
    }
}

/// Read with up to 3 retries on Windows ERROR_SHARING_VIOLATION (raw_os_error
/// == 32) or PermissionDenied — Goldberg JSON may be open-for-write briefly.
/// Mirrors the goldberg.rs::read_with_retry pattern (PATTERNS.md).
async fn read_with_retry(path: &std::path::Path) -> anyhow::Result<String> {
    let mut last_err: Option<std::io::Error> = None;
    for _ in 0..3 {
        match std::fs::read_to_string(path) {
            Ok(s) => return Ok(s),
            Err(e)
                if e.kind() == std::io::ErrorKind::PermissionDenied
                    || matches!(e.raw_os_error(), Some(32) | Some(33)) =>
            {
                last_err = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            Err(e) => return Err(e.into()),
        }
    }
    match last_err {
        Some(e) => Err(e.into()),
        None => Err(anyhow::anyhow!("read_with_retry: 0 attempts; refusing")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::SqliteStore;

    #[test]
    fn classify_tier_rare_when_below_10() {
        assert_eq!(classify_tier(Some(9.99)), "rare");
        assert_eq!(classify_tier(Some(0.0)), "rare");
        assert_eq!(classify_tier(Some(5.0)), "rare");
    }

    #[test]
    fn classify_tier_standard_when_at_or_above_10() {
        assert_eq!(classify_tier(Some(10.0)), "standard");
        assert_eq!(classify_tier(Some(50.5)), "standard");
        assert_eq!(classify_tier(Some(100.0)), "standard");
    }

    #[test]
    fn classify_tier_standard_when_unavailable() {
        // D-07 graceful degrade.
        assert_eq!(classify_tier(None), "standard");
    }

    #[tokio::test]
    async fn lookup_returns_none_for_uncached() {
        let store = Arc::new(SqliteStore::open_in_memory().unwrap());
        let sc = SchemaCache::new(store).unwrap();
        assert!(sc.lookup(480, "ACH_X").is_none());
    }

    #[tokio::test]
    async fn lookup_returns_row_after_upsert() {
        let store = Arc::new(SqliteStore::open_in_memory().unwrap());
        // Direct cache write to simulate prior session's resolve() output.
        store
            .with_conn(|c| {
                cache::upsert_schema(
                    c,
                    &cache::SchemaCacheRow {
                        app_id: 480,
                        ach_api_name: "ACH_X".into(),
                        display_name: Some("Got X".into()),
                        description: Some("did x".into()),
                        icon_path: None,
                        hidden: false,
                        global_pct: Some(7.5),
                        cached_at: 1700000000,
                    },
                )
            })
            .unwrap();
        let sc = SchemaCache::new(store).unwrap();
        let got = sc.lookup(480, "ACH_X").unwrap();
        assert_eq!(got.display_name.as_deref(), Some("Got X"));
        assert_eq!(got.global_pct, Some(7.5));
        // Tier classification on this row is rare (7.5 < 10).
        assert_eq!(classify_tier(got.global_pct), "rare");
    }

    #[tokio::test]
    async fn list_for_app_returns_empty_for_unknown() {
        let store = Arc::new(SqliteStore::open_in_memory().unwrap());
        let sc = SchemaCache::new(store).unwrap();
        assert!(sc.list_for_app(480).is_empty());
    }

    #[tokio::test]
    async fn list_for_app_returns_ordered_rows() {
        let store = Arc::new(SqliteStore::open_in_memory().unwrap());
        for n in ["ACH_C", "ACH_A", "ACH_B"] {
            store
                .with_conn(|c| {
                    cache::upsert_schema(
                        c,
                        &cache::SchemaCacheRow {
                            app_id: 480,
                            ach_api_name: n.into(),
                            display_name: None,
                            description: None,
                            icon_path: None,
                            hidden: false,
                            global_pct: None,
                            cached_at: 0,
                        },
                    )
                })
                .unwrap();
        }
        let sc = SchemaCache::new(store).unwrap();
        let list = sc.list_for_app(480);
        let names: Vec<_> = list.iter().map(|s| s.ach_api_name.as_str()).collect();
        assert_eq!(names, vec!["ACH_A", "ACH_B", "ACH_C"]);
    }
}
