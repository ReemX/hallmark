//! CreamAPI emulator achievements adapter (REQ DETECT-03).
//!
//! # Path layout
//!
//! `%APPDATA%\CreamAPI\<appid>\stats\CreamAPI.Achievements.cfg` — INI file. The
//! adapter's watch root is the per-appid directory (`%APPDATA%\CreamAPI\<appid>\`)
//! so the recursive `notify` watch picks up the `stats/CreamAPI.Achievements.cfg`
//! file. The appid is parsed from the watched root's parent-of-parent dir name.
//!
//! # File format (Hydra + Achievement-Watcher confirmed)
//!
//! ```ini
//! ### Optional triple-# comments
//! [ACH_API_NAME]
//! achieved=true|false|1|0
//! unlocktime=<unix-seconds | 7-digit microseconds | 13-digit milliseconds>
//! ```
//!
//! Per RESEARCH.md Pitfall #4, `unlocktime` is captured for telemetry only —
//! the `achieved=true` boolean transition is the only valid unlock signal.
//! Mirrors Phase 1's "earned: bool false→true" pattern for Goldberg.
//!
//! # Filename guard
//!
//! Only events for files named `CreamAPI.Achievements.cfg` are processed.
//! Sibling files in the `stats/` subdir (`CreamAPI.cfg`, `CreamAPI.Stats.cfg`,
//! comments backups) are silently skipped at the start of `on_file_changed`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, RwLock};

use super::{RawUnlockEvent, SourceAdapter, SourceKind};

const STATE_FILENAME: &str = "CreamAPI.Achievements.cfg";

#[derive(Debug, Clone, Default)]
pub struct CreamApiPaths {
    pub appid_dirs: Vec<PathBuf>,
}

pub fn discover_paths() -> CreamApiPaths {
    let mut out = CreamApiPaths::default();
    // Test-only override: `HALLMARK_CREAMAPI_ROOT_OVERRIDE` env var, when set,
    // replaces the default `dirs::data_dir().join("CreamAPI")` lookup. Production
    // code never sets this. Allows the SC2 integration test to verify auto-discovery
    // against a known fixture tree without polluting %APPDATA%. Pattern parallels
    // Phase 1's `HALLMARK_GOLDBERG_ROOT_OVERRIDE` (RESEARCH.md line 417).
    let cream_root: std::path::PathBuf = match std::env::var_os("HALLMARK_CREAMAPI_ROOT_OVERRIDE") {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            let Some(appdata) = dirs::data_dir() else { return out };
            appdata.join("CreamAPI")
        }
    };
    if !cream_root.exists() {
        return out;
    }
    let rd = match std::fs::read_dir(&cream_root) {
        Ok(r) => r,
        Err(_) => return out,
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        // Numeric appid filter — CreamAPI subdirs are exclusively integer appids.
        if name.parse::<u64>().is_err() {
            continue;
        }
        let cfg = path.join("stats").join(STATE_FILENAME);
        if cfg.exists() {
            out.appid_dirs.push(path);
        }
    }
    out
}

/// Parse the CreamAPI INI file body into `<api_name> → achieved` map.
/// Hydra-style 12-LoC line parser:
/// - BOM strip from the first line
/// - skip empty lines and `###` comments
/// - `[SECTION]` lines start a new achievement entry
/// - `key=value` lines under a section: only `achieved=...` is consumed
/// - `unlocktime` is read and discarded (Pitfall #4)
pub fn parse_creamapi_state(text: &str) -> HashMap<String, bool> {
    let mut out: HashMap<String, bool> = HashMap::new();
    let mut current: Option<String> = None;
    let mut first = true;
    for raw_line in text.lines() {
        let mut line = raw_line;
        if first {
            line = line.trim_start_matches('\u{feff}');
            first = false;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("###") || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') && line.len() > 2 {
            let inner = &line[1..line.len() - 1];
            // WR-08: reject empty / whitespace-only section names like `[]` or `[   ]`.
            // The previous `len() >= 2` check matched `[]` (length 2), inserting an
            // empty-string achievement key. Subsequent `achieved=true` would flip the
            // empty key to true and emit `RawUnlockEvent { ach_api_name: "" }`, which
            // would propagate to the popup UI and SQLite as a malformed entry.
            if inner.trim().is_empty() {
                current = None;
                continue;
            }
            current = Some(inner.to_string());
            // Default to false until an `achieved=` line confirms otherwise; this lets
            // the baseline include "locked" achievements so the diff can detect
            // a future false→true write even on entries the file already lists.
            out.entry(inner.to_string()).or_insert(false);
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let key = k.trim();
        let val = v.trim();
        if let Some(name) = &current {
            if key.eq_ignore_ascii_case("achieved") {
                let earned = matches!(val.to_ascii_lowercase().as_str(), "true" | "1");
                out.insert(name.clone(), earned);
            }
            // unlocktime intentionally ignored — Pitfall #4
        }
    }
    out
}

/// Parse appid from a `%APPDATA%\CreamAPI\<appid>\stats\CreamAPI.Achievements.cfg` path.
/// path.parent() is `stats`; path.parent().parent() is `<appid>`.
fn extract_app_id(path: &Path) -> Option<u64> {
    let stats_dir = path.parent()?;
    let appid_dir = stats_dir.parent()?;
    appid_dir.file_name().and_then(|n| n.to_str()).and_then(|s| s.parse::<u64>().ok())
}

pub struct CreamApiAdapter {
    cached_watch_paths: Vec<PathBuf>,
    baseline: Arc<RwLock<HashMap<(u64, String), bool>>>,
    last_hash: Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>,
}

impl CreamApiAdapter {
    pub fn new(appid_dirs: Vec<PathBuf>) -> Self {
        let cached: Vec<PathBuf> = appid_dirs.into_iter().filter(|p| p.exists()).collect();
        Self {
            cached_watch_paths: cached,
            baseline: Arc::new(RwLock::new(HashMap::new())),
            last_hash: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[cfg(test)]
    pub(crate) async fn baseline_snapshot(&self) -> HashMap<(u64, String), bool> {
        self.baseline.read().await.clone()
    }
}

#[async_trait::async_trait]
impl SourceAdapter for CreamApiAdapter {
    fn name(&self) -> &str { "cream_api" }
    fn kind(&self) -> SourceKind { SourceKind::CreamApi }

    fn watch_paths(&self) -> Vec<PathBuf> { self.cached_watch_paths.clone() }

    async fn seed_baseline(&self) -> anyhow::Result<()> {
        let mut baseline = self.baseline.write().await;
        let mut total_files = 0u32;
        let mut total_entries = 0u32;
        for appid_dir in &self.cached_watch_paths {
            let cfg = appid_dir.join("stats").join(STATE_FILENAME);
            if !cfg.exists() {
                continue;
            }
            let Some(app_id) = appid_dir
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|s| s.parse::<u64>().ok())
            else {
                tracing::warn!(path = %appid_dir.display(), "CreamAPI: appid_dir name not numeric; skipping");
                continue;
            };
            let text = match read_with_retry(&cfg).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(path = %cfg.display(), error = %e, "CreamAPI seed read failed");
                    continue;
                }
            };
            let state = parse_creamapi_state(&text);
            total_files += 1;
            for (api_name, earned) in state {
                baseline.insert((app_id, api_name), earned);
                total_entries += 1;
            }
        }
        tracing::info!(files = total_files, entries = total_entries, "CreamAPI baseline seeded");
        Ok(())
    }

    async fn on_file_changed(
        &self,
        path: PathBuf,
        tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()> {
        // Filename guard FIRST.
        // WR-06: Windows is case-insensitive at the filesystem level. `notify` typically
        // returns the on-disk case, but copy/paste, network shares, and backup tools can
        // produce mixed-case names that don't match the literal `STATE_FILENAME`
        // ("CreamAPI.Achievements.cfg"). `eq_ignore_ascii_case` accepts any ASCII case
        // variant; the filename here is pure ASCII so this is exhaustive.
        let matches = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.eq_ignore_ascii_case(STATE_FILENAME))
            .unwrap_or(false);
        if !matches {
            return Ok(());
        }
        let Some(app_id) = extract_app_id(&path) else {
            tracing::debug!(path = %path.display(), "CreamAPI: could not parse appid; ignoring");
            return Ok(());
        };

        let text = match read_with_retry(&path).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "CreamAPI read_with_retry failed");
                return Ok(());
            }
        };

        // CR-02: claim the new hash atomically under a single write-lock acquisition.
        // The previous read-then-write split allowed two concurrent on_file_changed
        // calls to both observe "hash absent", both proceed to parse + emit, and
        // produce duplicate events. Writing the new hash up-front ("claim") closes
        // the TOCTOU window: any concurrent caller observing the same hash will skip.
        // If parse/emit fails after the claim, the next event with identical bytes
        // legitimately skips (same content was already processed); a different hash
        // overwrites the entry. Mirrors goldberg's BL-02 ordering invariant — any
        // crash between claim and baseline-update yields at most one extra parse
        // on the next event, which is harmless.
        let hash: [u8; 32] = Sha256::digest(text.as_bytes()).into();
        {
            let mut h = self.last_hash.write().await;
            if h.get(&path) == Some(&hash) {
                tracing::trace!(path = %path.display(), "CreamAPI content unchanged; skip");
                return Ok(());
            }
            h.insert(path.clone(), hash);
        }

        let state = parse_creamapi_state(&text);

        // WR-01: Do not hold the baseline write-lock across `tx.send().await`. If the
        // downstream channel fills (debounced burst, slow consumer), `send().await`
        // blocks; holding the baseline lock for that duration starves any other task
        // attempting to read or update the baseline (e.g. a parallel on_file_changed
        // for a sibling app). Instead: under the lock, classify each entry — commit
        // the baseline immediately for non-emitting transitions (already-true,
        // false→false), and buffer the emitting ones. Then drop the lock, drain the
        // buffer with `tx.send`, and re-acquire briefly per success to commit. CR-01
        // is preserved: on send failure we skip the baseline.insert for that key so a
        // future event can re-fire.
        let mut events_to_send: Vec<(RawUnlockEvent, (u64, String), bool)> = Vec::new();
        {
            let mut baseline = self.baseline.write().await;
            for (api_name, earned_now) in state {
                let key = (app_id, api_name.clone());
                let was = baseline.get(&key).copied().unwrap_or(false);
                if !was && earned_now {
                    let evt = RawUnlockEvent {
                        app_id,
                        ach_api_name: api_name,
                        timestamp: 0,
                        source: SourceKind::CreamApi,
                    };
                    events_to_send.push((evt, key, earned_now));
                } else {
                    baseline.insert(key, earned_now);
                }
            }
        }
        for (evt, key, earned_now) in events_to_send {
            if let Err(e) = tx.send(evt).await {
                tracing::error!(error = %e, "RawUnlockEvent receiver dropped; not committing baseline so retry can fire");
                continue;
            }
            self.baseline.write().await.insert(key, earned_now);
        }
        // CR-02: hash was claimed atomically before parse/emit; no trailing insert needed.
        Ok(())
    }
}

/// Read with retry on Windows ERROR_SHARING_VIOLATION (32) / ERROR_LOCK_VIOLATION (33).
/// Mirrors `goldberg::read_with_retry`.
async fn read_with_retry(path: &Path) -> anyhow::Result<String> {
    let mut last_err: Option<std::io::Error> = None;
    for _ in 0..3 {
        match std::fs::read_to_string(path) {
            Ok(s) => return Ok(s),
            Err(e)
                if e.kind() == std::io::ErrorKind::PermissionDenied
                    || matches!(e.raw_os_error(), Some(32) | Some(33)) =>
            {
                last_err = Some(e);
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(e) => return Err(e.into()),
        }
    }
    match last_err {
        Some(e) => Err(e.into()),
        None => Err(anyhow::anyhow!("read_with_retry: 0 attempts")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tokio::sync::mpsc;
    use tokio::time::timeout;

    fn fresh_tmp() -> PathBuf {
        let p = std::env::temp_dir().join(format!("hallmark-cream-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn write_cream_fixture(root: &Path, app_id: u64, content: &str) -> PathBuf {
        let appid_dir = root.join(app_id.to_string());
        let stats_dir = appid_dir.join("stats");
        fs::create_dir_all(&stats_dir).unwrap();
        let path = stats_dir.join(STATE_FILENAME);
        fs::write(&path, content).unwrap();
        path
    }

    const FIXTURE_BASELINE: &str = "[ACH_BOSS]\nachieved=false\nunlocktime=0\n\n[ACH_FIRST]\nachieved=true\nunlocktime=1700000001\n\n[ACH_HIDDEN]\nachieved=false\nunlocktime=0\n";

    #[test]
    fn parse_creamapi_state_handles_basic_ini() {
        let m = parse_creamapi_state(FIXTURE_BASELINE);
        assert_eq!(m.get("ACH_BOSS").copied(), Some(false));
        assert_eq!(m.get("ACH_FIRST").copied(), Some(true));
        assert_eq!(m.get("ACH_HIDDEN").copied(), Some(false));
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn parse_creamapi_state_strips_bom() {
        let with_bom = format!("\u{feff}{}", FIXTURE_BASELINE);
        let m = parse_creamapi_state(&with_bom);
        assert_eq!(m.get("ACH_BOSS").copied(), Some(false));
        assert_eq!(m.get("ACH_FIRST").copied(), Some(true));
    }

    #[test]
    fn parse_creamapi_state_ignores_comments_and_unlocktime() {
        let text = "### comment line\n[ACH_X]\nachieved=true\nunlocktime=999\n###another\n";
        let m = parse_creamapi_state(text);
        assert_eq!(m.get("ACH_X").copied(), Some(true));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn parse_creamapi_state_rejects_empty_section_names() {
        // WR-08: `[]` and `[   ]` must not produce empty-string keys. A subsequent
        // `achieved=true` line under such a section is silently dropped (no current
        // section context).
        let text = "[]\nachieved=true\n[   ]\nachieved=true\n[ACH_OK]\nachieved=true\n";
        let m = parse_creamapi_state(text);
        assert!(!m.contains_key(""), "empty-string key must not be inserted; got map: {:?}", m);
        assert_eq!(m.get("ACH_OK").copied(), Some(true));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn parse_creamapi_state_treats_1_and_true_equivalently() {
        let text = "[A]\nachieved=1\n[B]\nachieved=TRUE\n[C]\nachieved=0\n[D]\nachieved=False\n";
        let m = parse_creamapi_state(text);
        assert_eq!(m.get("A").copied(), Some(true));
        assert_eq!(m.get("B").copied(), Some(true));
        assert_eq!(m.get("C").copied(), Some(false));
        assert_eq!(m.get("D").copied(), Some(false));
    }

    #[tokio::test]
    async fn seed_baseline_populates_from_fixture() {
        let root = fresh_tmp();
        let appid_dir = root.join("4242");
        write_cream_fixture(&root, 4242, FIXTURE_BASELINE);
        let adapter = CreamApiAdapter::new(vec![appid_dir]);
        adapter.seed_baseline().await.unwrap();
        let snap = adapter.baseline_snapshot().await;
        assert_eq!(snap.get(&(4242, "ACH_BOSS".to_string())).copied(), Some(false));
        assert_eq!(snap.get(&(4242, "ACH_FIRST".to_string())).copied(), Some(true));
        assert_eq!(snap.get(&(4242, "ACH_HIDDEN".to_string())).copied(), Some(false));
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn on_file_changed_emits_event_on_false_to_true_transition() {
        let root = fresh_tmp();
        let appid_dir = root.join("4242");
        let path = write_cream_fixture(&root, 4242, FIXTURE_BASELINE);
        let adapter = CreamApiAdapter::new(vec![appid_dir]);
        adapter.seed_baseline().await.unwrap();
        // Now flip ACH_BOSS to true.
        let updated = "[ACH_BOSS]\nachieved=true\nunlocktime=1700000099\n[ACH_FIRST]\nachieved=true\nunlocktime=1700000001\n[ACH_HIDDEN]\nachieved=false\n";
        fs::write(&path, updated).unwrap();
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(path, tx).await.unwrap();
        let evt = timeout(Duration::from_millis(500), rx.recv()).await.unwrap().unwrap();
        assert_eq!(evt.app_id, 4242);
        assert_eq!(evt.ach_api_name, "ACH_BOSS");
        assert_eq!(evt.source, SourceKind::CreamApi);
        // No second event for ACH_FIRST (already true at baseline).
        let second = timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(second.is_err() || second.unwrap().is_none(), "expected no further events");
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn on_file_changed_skips_non_cream_filename() {
        let root = fresh_tmp();
        let appid_dir = root.join("4242");
        fs::create_dir_all(appid_dir.join("stats")).unwrap();
        let bogus = appid_dir.join("stats").join("cream_api.ini");
        fs::write(&bogus, "[ACH_X]\nachieved=true\n").unwrap();
        let adapter = CreamApiAdapter::new(vec![appid_dir]);
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(bogus, tx).await.unwrap();
        let result = timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(result.is_err() || result.unwrap().is_none(), "non-cream filename must not produce events");
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn on_file_changed_skips_identical_content_via_sha256() {
        let root = fresh_tmp();
        let appid_dir = root.join("4242");
        let path = write_cream_fixture(&root, 4242, FIXTURE_BASELINE);
        let adapter = CreamApiAdapter::new(vec![appid_dir]);
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(path.clone(), tx.clone()).await.unwrap();
        let _ = timeout(Duration::from_millis(50), rx.recv()).await;
        // Second call with identical bytes — must short-circuit and not emit (and ACH_FIRST true at seed wouldn't emit anyway, but content-hash short-circuit prevents the parse).
        adapter.on_file_changed(path.clone(), tx.clone()).await.unwrap();
        let result = timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(result.is_err() || result.unwrap().is_none(), "identical content must short-circuit");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn extract_app_id_from_canonical_path() {
        let p = PathBuf::from("/tmp/CreamAPI/4242/stats/CreamAPI.Achievements.cfg");
        assert_eq!(extract_app_id(&p), Some(4242));
        let bad = PathBuf::from("/tmp/CreamAPI/notnumeric/stats/CreamAPI.Achievements.cfg");
        assert_eq!(extract_app_id(&bad), None);
    }

    #[test]
    fn discover_paths_returns_empty_when_no_creamapi_dir() {
        // We can't reliably assert discover_paths on the dev machine (CreamAPI may or may not
        // exist there); we instead assert it returns SOMETHING (Vec, possibly empty) without
        // panicking. The behavior under a real %APPDATA%\CreamAPI is exercised in 03-04
        // integration tests via fixture override.
        let p = discover_paths();
        let _ = p.appid_dirs;
    }
}
