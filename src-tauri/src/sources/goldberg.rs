//! Goldberg / gbe_fork emulator state-file watcher.
//!
//! # State file vs schema file (PITFALLS.md #4)
//!
//! There are two `achievements.json` files in a Goldberg setup. We only read ONE:
//!
//! - **STATE file** (this adapter): `%APPDATA%\Goldberg SteamEmu Saves\<appid>\achievements.json`
//!   Shape: `{ "ACH_NAME": { "earned": bool, "earned_time": u64 } }` (object map).
//! - **SCHEMA file** (Phase 2): `<game-dir>\steam_settings\achievements.json`
//!   Shape: `[{ "name": "...", "displayName": "...", "description": "...", ... }]` (array).
//!
//! Phase 1 watches ONLY the state file. Mixing them up parses garbage.
//!
//! # Why `earned: bool` is the only valid unlock signal (PITFALLS.md #15)
//!
//! Goldberg writes `earned_time: 0` for "earned but timestamp unknown". A naive
//! `unlock_time > 0` check would treat these as never-earned and fire spurious events
//! after baseline. The boolean `earned` field's `false → true` transition is the
//! only correct unlock signal.
//!
//! # Appid resolution: directory parse + redirect_map fallback
//!
//! Two directory layouts exist:
//! 1. Default: `<save_root>/<appid>/achievements.json` — parent dir name parses as u64.
//! 2. Redirect: `D:\Game1\Save\achievements.json` — parent is "Save" (not numeric).
//!    Plan 03's `goldberg_redirect_map()` provides `parent_dir → appid`. The adapter
//!    consults this map when the directory parse fails.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, RwLock};

use super::{RawUnlockEvent, SourceAdapter, SourceKind};

/// One entry in the Goldberg state file.
#[derive(Deserialize, Debug, Clone)]
struct GoldbergEntry {
    earned: bool,
    /// May be 0 or absent — DO NOT use as unlock signal (PITFALLS.md #15).
    #[serde(default)]
    #[allow(dead_code)]
    earned_time: u64,
}

type StateMap = HashMap<String, bool>;

/// Goldberg / gbe_fork adapter. Watches one or more save roots; for each
/// `<root>/<appid>/achievements.json` change (or redirect-target change),
/// diffs current `earned` state against an in-memory baseline and emits a
/// `RawUnlockEvent` per `false → true` transition.
pub struct GoldbergAdapter {
    roots: Vec<PathBuf>,
    /// Map from redirect-target parent directory → appid. Populated by Plan 03's
    /// `paths::goldberg_redirect_map()`. Used as a fallback when a state file's
    /// parent directory name does not parse as u64.
    redirect_map: HashMap<PathBuf, u64>,
    baseline: Arc<RwLock<HashMap<(u64, String), bool>>>,
    last_hash: Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>,
}

impl GoldbergAdapter {
    /// `roots` are the default Goldberg save roots (e.g. `%APPDATA%\GSE Saves\`).
    /// `redirect_map` is `HashMap<PathBuf, u64>` mapping each redirect target's
    /// parent directory to its resolved appid (built by `paths::goldberg_redirect_map`).
    pub fn new(roots: Vec<PathBuf>, redirect_map: HashMap<PathBuf, u64>) -> Self {
        Self {
            roots,
            redirect_map,
            baseline: Arc::new(RwLock::new(HashMap::new())),
            last_hash: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Resolve appid for a state file path. Tries:
    /// 1. Numeric parse of `path.parent().file_name()` (default `<root>/<appid>/achievements.json` layout).
    /// 2. Lookup of `path.parent()` in `redirect_map` (redirect layout where parent is e.g. "Save").
    /// Returns None if neither succeeds; caller logs and skips.
    fn extract_app_id(&self, path: &Path) -> Option<u64> {
        let parent = path.parent()?;
        // Step 1: numeric directory parse
        if let Some(name) = parent.file_name().and_then(|n| n.to_str()) {
            if let Ok(id) = name.parse::<u64>() {
                return Some(id);
            }
        }
        // Step 2: redirect_map fallback
        self.redirect_map.get(parent).copied()
    }

    fn parse_state(json: &str) -> anyhow::Result<StateMap> {
        let raw: HashMap<String, GoldbergEntry> = serde_json::from_str(json)?;
        Ok(raw.into_iter().map(|(k, v)| (k, v.earned)).collect())
    }

    /// Reentrant baseline read for tests — exposed in-crate.
    #[cfg(test)]
    pub(crate) async fn baseline_snapshot(&self) -> HashMap<(u64, String), bool> {
        self.baseline.read().await.clone()
    }
}

#[async_trait::async_trait]
impl SourceAdapter for GoldbergAdapter {
    fn name(&self) -> &str {
        "goldberg"
    }
    fn kind(&self) -> SourceKind {
        SourceKind::Goldberg
    }

    fn watch_paths(&self) -> Vec<PathBuf> {
        // Watch both default roots AND redirect-target parent dirs (the latter via redirect_map keys).
        let mut paths: Vec<PathBuf> = self.roots.iter().filter(|p| p.exists()).cloned().collect();
        for redirect_parent in self.redirect_map.keys() {
            if redirect_parent.exists() && !paths.contains(redirect_parent) {
                paths.push(redirect_parent.clone());
            }
        }
        paths
    }

    async fn seed_baseline(&self) -> anyhow::Result<()> {
        let mut baseline = self.baseline.write().await;
        let mut total_files = 0u32;
        let mut total_entries = 0u32;

        // Pass 1 — default roots, layout `<root>/<appid>/achievements.json`.
        for root in &self.roots {
            if !root.exists() {
                tracing::warn!(root = %root.display(), "Goldberg root does not exist; skipping");
                continue;
            }
            // max_depth(2) hits exactly <root>/<appid>/achievements.json without
            // chasing arbitrarily deep subdirs.
            for entry in walkdir::WalkDir::new(root).max_depth(2) {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!(root = %root.display(), error = %e, "walkdir error during seed");
                        continue;
                    }
                };
                if !entry.file_type().is_file() {
                    continue;
                }
                if entry.file_name() != "achievements.json" {
                    continue;
                }

                let path = entry.path();
                let Some(app_id) = self.extract_app_id(path) else {
                    tracing::warn!(path = %path.display(),
                        "could not resolve appid (numeric parse failed and not in redirect_map); skipping");
                    continue;
                };
                let json = match read_with_retry(path) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(path = %path.display(), error = %e, "seed read failed; skip");
                        continue;
                    }
                };
                let state = match Self::parse_state(&json) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(path = %path.display(), error = %e, "seed parse failed; skip");
                        continue;
                    }
                };
                total_files += 1;
                for (api_name, earned) in state {
                    baseline.insert((app_id, api_name), earned);
                    total_entries += 1;
                }
            }
        }

        // Pass 2 — redirect targets, layout `<redirect_parent>/achievements.json`.
        // Each redirect_map key IS the redirect target directory; the achievements.json
        // sits directly inside it (no per-appid subdir).
        for (redirect_parent, &app_id) in &self.redirect_map {
            let candidate = redirect_parent.join("achievements.json");
            if !candidate.exists() {
                continue;
            }
            let json = match read_with_retry(&candidate) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(path = %candidate.display(), error = %e,
                        "redirect-target seed read failed; skip");
                    continue;
                }
            };
            let state = match Self::parse_state(&json) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(path = %candidate.display(), error = %e,
                        "redirect-target seed parse failed; skip");
                    continue;
                }
            };
            total_files += 1;
            for (api_name, earned) in state {
                baseline.insert((app_id, api_name), earned);
                total_entries += 1;
            }
        }

        tracing::info!(
            files = total_files,
            entries = total_entries,
            roots = self.roots.len(),
            redirects = self.redirect_map.len(),
            "Goldberg baseline seeded"
        );
        Ok(())
    }

    async fn on_file_changed(
        &self,
        path: PathBuf,
        tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()> {
        // Filter: events from the recursive watch may include sibling files in the
        // appid directory. We only care about achievements.json itself.
        if path.file_name().and_then(|n| n.to_str()) != Some("achievements.json") {
            return Ok(());
        }
        let Some(app_id) = self.extract_app_id(&path) else {
            tracing::debug!(path = %path.display(),
                "could not resolve appid for event path; ignoring");
            return Ok(());
        };

        // Read with retry (PITFALLS.md #3 — file may be locked while emulator writes).
        let json = match read_with_retry(&path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "read_with_retry failed");
                return Ok(());
            }
        };

        // Content-hash equality (REQ DETECT-06 second layer).
        let hash: [u8; 32] = Sha256::digest(json.as_bytes()).into();
        {
            let mut hashes = self.last_hash.write().await;
            if hashes.get(&path) == Some(&hash) {
                tracing::trace!(path = %path.display(), "content unchanged (hash match); skip");
                return Ok(());
            }
            hashes.insert(path.clone(), hash);
        }

        let state = match Self::parse_state(&json) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "state file parse failed");
                return Ok(());
            }
        };

        // Diff against baseline. Order: read → diff → emit → THEN update baseline.
        let mut baseline = self.baseline.write().await;
        for (api_name, earned_now) in state {
            let key = (app_id, api_name.clone());
            let was = baseline.get(&key).copied().unwrap_or(false);
            if !was && earned_now {
                let evt = RawUnlockEvent {
                    app_id,
                    ach_api_name: api_name,
                    timestamp: 0, // wall-clock stamping happens downstream
                    source: SourceKind::Goldberg,
                };
                if tx.send(evt).await.is_err() {
                    tracing::error!("RawUnlockEvent receiver dropped; pipeline shutting down?");
                }
            }
            baseline.insert(key, earned_now);
        }
        Ok(())
    }
}

/// File read with retry on Windows sharing-violation (PITFALLS.md #3).
/// Goldberg writes the state file open-write-close; we may hit it mid-write.
fn read_with_retry(path: &Path) -> anyhow::Result<String> {
    let mut last_err: Option<std::io::Error> = None;
    for _ in 0..3 {
        match std::fs::read_to_string(path) {
            Ok(s) => return Ok(s),
            Err(e)
                if e.kind() == std::io::ErrorKind::PermissionDenied
                    || e.raw_os_error() == Some(32) /* ERROR_SHARING_VIOLATION */ =>
            {
                last_err = Some(e);
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(e.into()),
        }
    }
    Err(last_err.unwrap().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tokio::sync::mpsc;
    use tokio::time::timeout;

    fn fresh_tmp() -> PathBuf {
        let p = std::env::temp_dir().join(format!("hallmark-gold-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn write_fixture(root: &Path, app_id: u64, content: &str) -> PathBuf {
        let dir = root.join(app_id.to_string());
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("achievements.json");
        fs::write(&path, content).unwrap();
        path
    }

    const FIXTURE_BASELINE: &str = r#"{
        "ACH_WIN_ONE_GAME":     { "earned": true,  "earned_time": 1700000001 },
        "ACH_WIN_100_GAMES":    { "earned": false, "earned_time": 0 },
        "ACH_TRAVEL_FAR_ACCUM": { "earned": false, "earned_time": 0 },
        "ACH_UNKNOWN_TIMESTAMP":{ "earned": true,  "earned_time": 0 }
    }"#;

    #[tokio::test]
    async fn seed_baseline_populates_from_fixture() {
        let root = fresh_tmp();
        write_fixture(&root, 480, FIXTURE_BASELINE);

        let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
        adapter.seed_baseline().await.unwrap();

        let snap = adapter.baseline_snapshot().await;
        assert_eq!(snap.len(), 4);
        assert_eq!(
            snap.get(&(480, "ACH_WIN_ONE_GAME".to_string())),
            Some(&true)
        );
        assert_eq!(
            snap.get(&(480, "ACH_WIN_100_GAMES".to_string())),
            Some(&false)
        );
        assert_eq!(
            snap.get(&(480, "ACH_TRAVEL_FAR_ACCUM".to_string())),
            Some(&false)
        );
        assert_eq!(
            snap.get(&(480, "ACH_UNKNOWN_TIMESTAMP".to_string())),
            Some(&true)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn on_file_changed_emits_event_on_false_to_true_transition() {
        let root = fresh_tmp();
        let path = write_fixture(&root, 480, FIXTURE_BASELINE);

        let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
        adapter.seed_baseline().await.unwrap();

        // Flip ACH_WIN_100_GAMES from false to true
        let mutated = FIXTURE_BASELINE.replace(
            r#""ACH_WIN_100_GAMES":    { "earned": false, "earned_time": 0 }"#,
            r#""ACH_WIN_100_GAMES":    { "earned": true,  "earned_time": 1700000999 }"#,
        );
        assert_ne!(
            mutated, FIXTURE_BASELINE,
            "test mutation must change content"
        );
        fs::write(&path, &mutated).unwrap();

        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(path.clone(), tx).await.unwrap();

        let evt = timeout(Duration::from_millis(200), rx.recv())
            .await
            .unwrap()
            .expect("expected exactly one event");
        assert_eq!(evt.app_id, 480);
        assert_eq!(evt.ach_api_name, "ACH_WIN_100_GAMES");
        assert_eq!(evt.source, SourceKind::Goldberg);

        // No additional events — channel should yield nothing more within 100ms.
        let none = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(
            none.is_err() || none.unwrap().is_none(),
            "no further events should arrive"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn on_file_changed_no_event_for_already_earned_at_seed() {
        let root = fresh_tmp();
        let path = write_fixture(&root, 480, FIXTURE_BASELINE);

        let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
        adapter.seed_baseline().await.unwrap();

        // Re-read the SAME file (simulating a debounced no-op event).
        // Mutate content slightly so the hash differs (force the diff path).
        // ACH_WIN_ONE_GAME is already true — re-emitting `earned: true` should produce no event.
        let cosmetic = FIXTURE_BASELINE.replace("1700000001", "1700000002");
        fs::write(&path, &cosmetic).unwrap();

        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(path, tx).await.unwrap();
        let none = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(
            none.is_err() || none.unwrap().is_none(),
            "no event for already-earned achievement"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn on_file_changed_no_event_for_earned_time_zero_with_earned_true() {
        // Specifically PITFALLS.md #15: ACH_UNKNOWN_TIMESTAMP has earned_time=0 but earned=true.
        // After baseline seeding, no event should fire on a re-read.
        let root = fresh_tmp();
        let path = write_fixture(&root, 480, FIXTURE_BASELINE);

        let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
        adapter.seed_baseline().await.unwrap();

        // Force hash difference by adding whitespace, but keep state semantically identical.
        fs::write(&path, format!("{}\n  ", FIXTURE_BASELINE)).unwrap();

        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(path, tx).await.unwrap();
        let none = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(
            none.is_err() || none.unwrap().is_none(),
            "earned_time:0 + earned:true after seed must NOT emit (PITFALLS #15)"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn on_file_changed_skips_identical_content_via_sha256() {
        let root = fresh_tmp();
        let path = write_fixture(&root, 480, FIXTURE_BASELINE);
        let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
        adapter.seed_baseline().await.unwrap();

        // Without modifying file at all, call on_file_changed twice.
        // First call hashes + diffs (no event because everything matches baseline).
        // Second call short-circuits on hash equality.
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter
            .on_file_changed(path.clone(), tx.clone())
            .await
            .unwrap();
        adapter.on_file_changed(path, tx).await.unwrap();

        let none = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(none.is_err() || none.unwrap().is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn on_file_changed_no_event_when_filename_not_achievements_json() {
        let root = fresh_tmp();
        let dir = root.join("480");
        fs::create_dir_all(&dir).unwrap();
        let other = dir.join("cooldown.txt");
        fs::write(&other, "irrelevant").unwrap();

        let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(other, tx).await.unwrap();

        let none = timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(none.is_err() || none.unwrap().is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn extract_app_id_returns_none_for_unknown_non_numeric_dir() {
        let root = fresh_tmp();
        let dir = root.join("notanumber");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("achievements.json");
        fs::write(&path, "{}").unwrap();

        let adapter = GoldbergAdapter::new(vec![root.clone()], HashMap::new());
        assert_eq!(
            adapter.extract_app_id(&path),
            None,
            "non-numeric dir without redirect_map entry should return None"
        );
        adapter.seed_baseline().await.unwrap();
        let snap = adapter.baseline_snapshot().await;
        assert!(
            snap.is_empty(),
            "non-numeric dir should produce no baseline entries"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn extract_app_id_uses_redirect_map_for_non_numeric_parent() {
        let root = fresh_tmp();
        let save_dir = root.join("Save");
        fs::create_dir_all(&save_dir).unwrap();
        let path = save_dir.join("achievements.json");
        fs::write(&path, "{}").unwrap();

        let mut redirect_map = HashMap::new();
        redirect_map.insert(save_dir.clone(), 12345);

        let adapter = GoldbergAdapter::new(vec![], redirect_map);
        assert_eq!(
            adapter.extract_app_id(&path),
            Some(12345),
            "redirect_map should resolve appid for non-numeric parent"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn seed_baseline_reads_redirect_targets() {
        let root = fresh_tmp();
        let save_dir = root.join("CustomSave");
        fs::create_dir_all(&save_dir).unwrap();
        fs::write(save_dir.join("achievements.json"), FIXTURE_BASELINE).unwrap();

        let mut redirect_map = HashMap::new();
        redirect_map.insert(save_dir.clone(), 67890);

        let adapter = GoldbergAdapter::new(vec![], redirect_map);
        adapter.seed_baseline().await.unwrap();

        let snap = adapter.baseline_snapshot().await;
        assert_eq!(
            snap.len(),
            4,
            "expected 4 entries from redirect target seed; got {:?}",
            snap
        );
        assert_eq!(
            snap.get(&(67890, "ACH_WIN_ONE_GAME".to_string())),
            Some(&true)
        );
        assert_eq!(
            snap.get(&(67890, "ACH_WIN_100_GAMES".to_string())),
            Some(&false)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn on_file_changed_emits_event_via_redirect_map_lookup() {
        let root = fresh_tmp();
        let save_dir = root.join("Save");
        fs::create_dir_all(&save_dir).unwrap();
        let path = save_dir.join("achievements.json");
        fs::write(&path, FIXTURE_BASELINE).unwrap();

        let mut redirect_map = HashMap::new();
        redirect_map.insert(save_dir.clone(), 99999);

        let adapter = GoldbergAdapter::new(vec![], redirect_map);
        adapter.seed_baseline().await.unwrap();

        // Flip ACH_WIN_100_GAMES
        let mutated = FIXTURE_BASELINE.replace(
            r#""ACH_WIN_100_GAMES":    { "earned": false, "earned_time": 0 }"#,
            r#""ACH_WIN_100_GAMES":    { "earned": true,  "earned_time": 1700000999 }"#,
        );
        fs::write(&path, &mutated).unwrap();

        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(path.clone(), tx).await.unwrap();

        let evt = timeout(Duration::from_millis(200), rx.recv())
            .await
            .unwrap()
            .expect("expected one event via redirect_map lookup");
        assert_eq!(evt.app_id, 99999, "appid should come from redirect_map");
        assert_eq!(evt.ach_api_name, "ACH_WIN_100_GAMES");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn integration_full_cycle_against_real_disk() {
        let root = fresh_tmp();
        let path = write_fixture(&root, 480, FIXTURE_BASELINE);
        let adapter = Arc::new(GoldbergAdapter::new(vec![root.clone()], HashMap::new()));

        adapter.seed_baseline().await.unwrap();
        // Flip TWO entries in one write: ACH_WIN_100_GAMES and ACH_TRAVEL_FAR_ACCUM
        let mutated = FIXTURE_BASELINE
            .replace(
                r#""ACH_WIN_100_GAMES":    { "earned": false, "earned_time": 0 }"#,
                r#""ACH_WIN_100_GAMES":    { "earned": true,  "earned_time": 1700001000 }"#,
            )
            .replace(
                r#""ACH_TRAVEL_FAR_ACCUM": { "earned": false, "earned_time": 0 }"#,
                r#""ACH_TRAVEL_FAR_ACCUM": { "earned": true,  "earned_time": 1700001001 }"#,
            );
        fs::write(&path, &mutated).unwrap();

        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(path, tx).await.unwrap();

        let mut received = Vec::new();
        for _ in 0..2 {
            match timeout(Duration::from_millis(200), rx.recv()).await {
                Ok(Some(e)) => received.push(e),
                _ => break,
            }
        }
        assert_eq!(
            received.len(),
            2,
            "expected both transitions to emit; got {:?}",
            received
        );
        let names: Vec<&str> = received.iter().map(|e| e.ach_api_name.as_str()).collect();
        assert!(names.contains(&"ACH_WIN_100_GAMES"));
        assert!(names.contains(&"ACH_TRAVEL_FAR_ACCUM"));

        let _ = fs::remove_dir_all(&root);
    }
}
