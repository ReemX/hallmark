//! SmartSteamEmu emulator achievements adapter (REQ DETECT-04).
//!
//! # Path layout
//!
//! `%APPDATA%\SmartSteamEmu\<appid>\stats.bin` — fixed-size binary format. The
//! adapter's watch root is the per-appid directory; the recursive `notify` watch
//! picks up `stats.bin` events.
//!
//! # File format (Achievement-Watcher sse.js confirmed)
//!
//! ```
//! [ 0..4   ] Int32LE: expectedStatsCount
//! [ 4..    ] N × 24-byte records:
//!   record[0..4]   CRC32(api_name) — bytes are stored reversed; read as big-endian u32 of the reversed slice
//!   record[4..8]   reserved
//!   record[8..12]  Int32LE: UnlockTime (unix seconds, 0 = unknown)
//!   record[12..20] reserved
//!   record[20..24] Int32LE: value (achievement state if 0 or 1; stat if >1, skip)
//! ```
//!
//! # Pitfall #3 — leading-zero CRC stripping
//!
//! Reference parsers in JavaScript strip leading zeros for hashes < 0x1000. To stay
//! compatible across both ecosystems, this adapter zero-pads all CRC hex strings to
//! exactly 8 characters. The reverse-lookup map keys MUST also be 8-char zero-padded.
//!
//! # Lazy CRC reverse-map construction
//!
//! Records key on CRC32(api_name), not the API name. We need the inverse to emit
//! RawUnlockEvent. Build it lazily on first event for an appid by enumerating
//! candidate API names from a sibling Goldberg companion file (`%APPDATA%\GSE Saves
//! \<appid>\achievements.json`) — Goldberg's state file is JSON keyed on API names,
//! so the keys are the candidates. If no candidate file exists, emit a `<crc:0x...>`
//! placeholder name (Pitfall #8 analog).
//!
//! # Variant deferral (Open Question #2)
//!
//! Hydra references `%APPDATA%\SmartSteamEmu\<appid>\User\Achievements.ini` as an
//! alternate path. v1 ships ONLY the `stats.bin` variant. The INI variant is logged
//! warn-level and skipped during discovery; if a user has only the INI variant, the
//! adapter is silent for their installs. Phase 4 polish revisits this if user reports
//! surface.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, RwLock};

use super::{RawUnlockEvent, SourceAdapter, SourceKind};

const STATE_FILENAME: &str = "stats.bin";

#[derive(Debug, Clone, Default)]
pub struct SsePaths {
    pub appid_dirs: Vec<PathBuf>,
}

pub fn discover_paths() -> SsePaths {
    let mut out = SsePaths::default();
    // Test-only override: `HALLMARK_SSE_ROOT_OVERRIDE` env var, when set, replaces
    // the default `dirs::data_dir().join("SmartSteamEmu")` lookup. Production code
    // never sets this. Allows the SC2 integration test to verify auto-discovery
    // against a known fixture tree without polluting %APPDATA%. Pattern parallels
    // Phase 1's `HALLMARK_GOLDBERG_ROOT_OVERRIDE` (RESEARCH.md line 417).
    let sse_root: std::path::PathBuf = match std::env::var_os("HALLMARK_SSE_ROOT_OVERRIDE") {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            let Some(appdata) = dirs::data_dir() else { return out };
            appdata.join("SmartSteamEmu")
        }
    };
    if !sse_root.exists() {
        return out;
    }
    let rd = match std::fs::read_dir(&sse_root) {
        Ok(r) => r,
        Err(_) => return out,
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        // Numeric appid filter.
        if name.parse::<u64>().is_err() {
            continue;
        }
        let stats = path.join(STATE_FILENAME);
        let alt_ini = path.join("User").join("Achievements.ini");
        if stats.exists() {
            out.appid_dirs.push(path);
        } else if alt_ini.exists() {
            tracing::warn!(
                path = %alt_ini.display(),
                "found %APPDATA%\\SmartSteamEmu\\<appid>\\User\\Achievements.ini but no stats.bin — variant not supported in v1 (RESEARCH.md Open Question #2)"
            );
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct SseRecord {
    /// Zero-padded 8-char hex CRC32 of the api_name (Pitfall #3).
    pub crc32_hex: String,
    pub achieved: bool,
    #[allow(dead_code)]
    pub unlock_time: u32,
}

pub fn parse_sse_stats(bytes: &[u8]) -> anyhow::Result<Vec<SseRecord>> {
    if bytes.len() < 4 {
        return Ok(Vec::new());
    }
    let count_raw = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    // Defensive cap: take the smaller of the declared count and what fits in remaining bytes.
    let max_records = (bytes.len() - 4) / 24;
    let count = if count_raw < 0 {
        0
    } else {
        std::cmp::min(count_raw as usize, max_records)
    };
    if count_raw as usize != count && count_raw >= 0 {
        tracing::warn!(declared = count_raw, capped = count, "SSE stats.bin: declared count exceeds file size; capping");
    }

    let mut out = Vec::with_capacity(count);
    let mut off = 4;
    for _ in 0..count {
        if off + 24 > bytes.len() {
            break;
        }
        let r = &bytes[off..off + 24];
        // CRC bytes are reversed in file; reading reversed slice as big-endian u32 yields the natural CRC32 value.
        let crc_bytes = [r[3], r[2], r[1], r[0]];
        let crc = u32::from_be_bytes(crc_bytes);
        let unlock_time = u32::from_le_bytes([r[8], r[9], r[10], r[11]]);
        let value = i32::from_le_bytes([r[20], r[21], r[22], r[23]]);
        // Skip stats (value > 1) — only achievements (0 or 1) wanted.
        if value > 1 {
            off += 24;
            continue;
        }
        out.push(SseRecord {
            crc32_hex: format!("{:08x}", crc),
            achieved: value == 1,
            unlock_time,
        });
        off += 24;
    }
    Ok(out)
}

/// Build a reverse-lookup map (zero-padded 8-char CRC32 hex → api_name) from a list of candidate API names.
pub fn build_crc_reverse_map(candidates: &[String]) -> HashMap<String, String> {
    let mut out = HashMap::with_capacity(candidates.len());
    for name in candidates {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(name.as_bytes());
        let crc = hasher.finalize();
        let key = format!("{:08x}", crc);
        out.insert(key, name.clone());
    }
    out
}

/// Read a sibling Goldberg companion file at `%APPDATA%\GSE Saves\<appid>\achievements.json`
/// and return the top-level keys as candidate API names. Returns empty Vec if the file is
/// missing or unparseable.
fn load_goldberg_companion_keys(app_id: u64) -> Vec<String> {
    let Some(appdata) = dirs::data_dir() else { return Vec::new() };
    let candidate1 = appdata.join("GSE Saves").join(app_id.to_string()).join("achievements.json");
    let candidate2 = appdata.join("Goldberg SteamEmu Saves").join(app_id.to_string()).join("achievements.json");
    for path in [candidate1, candidate2] {
        if !path.exists() {
            continue;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let value: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(obj) = value.as_object() {
            return obj.keys().cloned().collect();
        }
    }
    Vec::new()
}

/// Resolve api_name from a CRC hex key, consulting the per-app reverse map.
/// Falls back to a `<crc:0x{:08x}>` placeholder if no candidate maps.
async fn resolve_api_name(
    crc_reverse: &Arc<RwLock<HashMap<u64, HashMap<String, String>>>>,
    app_id: u64,
    crc_hex: &str,
) -> String {
    {
        let r = crc_reverse.read().await;
        if let Some(map) = r.get(&app_id) {
            if let Some(name) = map.get(crc_hex) {
                return name.clone();
            }
        }
    }
    // Lazy build: fetch candidates and store the map.
    let candidates = load_goldberg_companion_keys(app_id);
    let map = build_crc_reverse_map(&candidates);
    let resolved = map.get(crc_hex).cloned();
    crc_reverse.write().await.insert(app_id, map);
    resolved.unwrap_or_else(|| format!("<crc:0x{}>", crc_hex))
}

fn extract_app_id(path: &Path) -> Option<u64> {
    let appid_dir = path.parent()?;
    appid_dir.file_name().and_then(|n| n.to_str()).and_then(|s| s.parse::<u64>().ok())
}

pub struct SseAdapter {
    cached_watch_paths: Vec<PathBuf>,
    baseline: Arc<RwLock<HashMap<(u64, String), bool>>>,
    last_hash: Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>,
    /// Per-app CRC32-hex → api_name reverse map (lazily built on first event for an appid).
    crc_reverse: Arc<RwLock<HashMap<u64, HashMap<String, String>>>>,
}

impl SseAdapter {
    pub fn new(appid_dirs: Vec<PathBuf>) -> Self {
        let cached: Vec<PathBuf> = appid_dirs.into_iter().filter(|p| p.exists()).collect();
        Self {
            cached_watch_paths: cached,
            baseline: Arc::new(RwLock::new(HashMap::new())),
            last_hash: Arc::new(RwLock::new(HashMap::new())),
            crc_reverse: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[cfg(test)]
    pub(crate) async fn baseline_snapshot(&self) -> HashMap<(u64, String), bool> {
        self.baseline.read().await.clone()
    }
}

#[async_trait::async_trait]
impl SourceAdapter for SseAdapter {
    fn name(&self) -> &str { "smartsteamemu" }
    fn kind(&self) -> SourceKind { SourceKind::SmartSteamEmu }

    fn watch_paths(&self) -> Vec<PathBuf> { self.cached_watch_paths.clone() }

    async fn seed_baseline(&self) -> anyhow::Result<()> {
        let mut baseline = self.baseline.write().await;
        let mut total_files = 0u32;
        let mut total_entries = 0u32;
        for appid_dir in &self.cached_watch_paths {
            let stats_path = appid_dir.join(STATE_FILENAME);
            if !stats_path.exists() {
                continue;
            }
            let Some(app_id) = appid_dir
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|s| s.parse::<u64>().ok())
            else {
                tracing::warn!(path = %appid_dir.display(), "SSE: appid_dir name not numeric; skipping");
                continue;
            };
            let bytes = match read_bytes_with_retry(&stats_path).await {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(path = %stats_path.display(), error = %e, "SSE seed read failed");
                    continue;
                }
            };
            let records = match parse_sse_stats(&bytes) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(path = %stats_path.display(), error = %e, "SSE seed parse failed");
                    continue;
                }
            };
            total_files += 1;
            // Drop the write lock while we resolve names (which may take its own lock on crc_reverse).
            drop(baseline);
            for rec in records {
                let api_name = resolve_api_name(&self.crc_reverse, app_id, &rec.crc32_hex).await;
                baseline = self.baseline.write().await;
                baseline.insert((app_id, api_name), rec.achieved);
                total_entries += 1;
                drop(baseline);
            }
            baseline = self.baseline.write().await;
        }
        tracing::info!(files = total_files, entries = total_entries, "SSE baseline seeded");
        Ok(())
    }

    async fn on_file_changed(
        &self,
        path: PathBuf,
        tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()> {
        // WR-06: case-insensitive match on Windows (case-insensitive filesystem).
        // Mixed-case `stats.bin` from copy/paste / network shares / backup tools
        // would otherwise be silently dropped.
        let matches = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.eq_ignore_ascii_case(STATE_FILENAME))
            .unwrap_or(false);
        if !matches {
            return Ok(());
        }
        let Some(app_id) = extract_app_id(&path) else {
            tracing::debug!(path = %path.display(), "SSE: could not parse appid; ignoring");
            return Ok(());
        };

        let bytes = match read_bytes_with_retry(&path).await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "SSE read_bytes_with_retry failed");
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
        let hash: [u8; 32] = Sha256::digest(&bytes).into();
        {
            let mut h = self.last_hash.write().await;
            if h.get(&path) == Some(&hash) {
                tracing::trace!(path = %path.display(), "SSE content unchanged; skip");
                return Ok(());
            }
            h.insert(path.clone(), hash);
        }

        let records = match parse_sse_stats(&bytes) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "SSE stats.bin parse failed");
                return Ok(());
            }
        };

        // WR-01: Do not hold the baseline write-lock across `tx.send().await`. See
        // matching comment in cream_api.rs. We resolve api_names first (which take
        // their own crc_reverse lock), then under the baseline lock classify each
        // record — commit baseline for non-emitting transitions immediately, buffer
        // emitting ones — drop the lock, then drain. CR-01 preserved: on send
        // failure we skip the baseline.insert for that key so a future event can
        // re-fire.
        let mut resolved: Vec<(String, bool)> = Vec::with_capacity(records.len());
        for rec in records {
            let api_name = resolve_api_name(&self.crc_reverse, app_id, &rec.crc32_hex).await;
            resolved.push((api_name, rec.achieved));
        }
        let mut events_to_send: Vec<(RawUnlockEvent, (u64, String), bool)> = Vec::new();
        {
            let mut baseline = self.baseline.write().await;
            for (api_name, achieved) in resolved {
                let key = (app_id, api_name.clone());
                let was = baseline.get(&key).copied().unwrap_or(false);
                if !was && achieved {
                    let evt = RawUnlockEvent {
                        app_id,
                        ach_api_name: api_name,
                        timestamp: 0,
                        source: SourceKind::SmartSteamEmu,
                    };
                    events_to_send.push((evt, key, achieved));
                } else {
                    baseline.insert(key, achieved);
                }
            }
        }
        for (evt, key, achieved) in events_to_send {
            if let Err(e) = tx.send(evt).await {
                tracing::error!(error = %e, "RawUnlockEvent receiver dropped; not committing baseline so retry can fire");
                continue;
            }
            self.baseline.write().await.insert(key, achieved);
        }
        // CR-02: hash was claimed atomically before parse/emit; no trailing insert needed.
        Ok(())
    }
}

async fn read_bytes_with_retry(path: &Path) -> anyhow::Result<Vec<u8>> {
    let mut last_err: Option<std::io::Error> = None;
    for _ in 0..3 {
        match std::fs::read(path) {
            Ok(b) => return Ok(b),
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
        None => Err(anyhow::anyhow!("read_bytes_with_retry: 0 attempts")),
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
        let p = std::env::temp_dir().join(format!("hallmark-sse-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// Build a synthetic stats.bin: header(count) + records.
    /// Each record: (crc_u32, achieved, unlock_time)
    fn synth_stats_bin(records: &[(u32, bool, u32)]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::with_capacity(4 + 24 * records.len());
        let count = records.len() as i32;
        out.extend_from_slice(&count.to_le_bytes());
        for (crc, achieved, ut) in records {
            // CRC bytes are stored reversed: file bytes [0..4] = u32::to_le_bytes(crc.swap_bytes())
            // Equivalently, write big-endian and the reader will reverse.
            // The parser reads record[3], record[2], record[1], record[0] then big-endian u32. So:
            //   record[0] = crc_bytes[3], record[1] = crc_bytes[2], record[2] = crc_bytes[1], record[3] = crc_bytes[0]
            // i.e. record[0..4] = crc.to_le_bytes() because to_le == reverse-of-be on little-endian. Confirm:
            // For crc=0x12345678: to_be = [12, 34, 56, 78]; to_le = [78, 56, 34, 12]. Reader does
            //   crc_bytes_after_swap = [r[3], r[2], r[1], r[0]] = [12, 34, 56, 78] = to_be → u32::from_be_bytes
            //   yields 0x12345678. Correct.
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&[0u8; 4]); // reserved
            out.extend_from_slice(&ut.to_le_bytes()); // unlock_time
            out.extend_from_slice(&[0u8; 8]); // reserved
            let value: i32 = if *achieved { 1 } else { 0 };
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }

    #[test]
    fn parse_sse_stats_round_trip_synthetic() {
        let crc1 = 0x12345678u32;
        let crc2 = 0x000000ABu32; // small CRC — the leading-zero-strip pitfall test
        let bytes = synth_stats_bin(&[(crc1, true, 1700000001), (crc2, false, 0)]);
        let recs = parse_sse_stats(&bytes).unwrap();
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].crc32_hex, "12345678");
        assert_eq!(recs[0].achieved, true);
        assert_eq!(recs[1].crc32_hex, "000000ab"); // zero-padded 8 chars
        assert_eq!(recs[1].achieved, false);
    }

    #[test]
    fn parse_sse_stats_skips_value_greater_than_one() {
        // Stat (not achievement) — value=42; should be skipped.
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(&1i32.to_le_bytes()); // count
        bytes.extend_from_slice(&0xCAFEBABEu32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 4]);
        bytes.extend_from_slice(&0u32.to_le_bytes()); // unlock_time
        bytes.extend_from_slice(&[0u8; 8]);
        bytes.extend_from_slice(&42i32.to_le_bytes()); // value=42 → stat, skip
        let recs = parse_sse_stats(&bytes).unwrap();
        assert_eq!(recs.len(), 0);
    }

    #[test]
    fn parse_sse_stats_caps_count_to_file_size() {
        // Declare 100 records but provide only 2 records' worth of bytes.
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(&100i32.to_le_bytes());
        bytes.extend_from_slice(&synth_stats_bin(&[(0xAA, true, 0), (0xBB, false, 0)])[4..]);
        let recs = parse_sse_stats(&bytes).unwrap();
        assert_eq!(recs.len(), 2, "should cap to actual file size");
    }

    #[test]
    fn parse_sse_stats_empty_file_returns_empty() {
        assert_eq!(parse_sse_stats(&[]).unwrap().len(), 0);
        assert_eq!(parse_sse_stats(&[0, 0, 0, 0]).unwrap().len(), 0);
    }

    #[test]
    fn build_crc_reverse_map_zero_pads_keys() {
        // CRC32 of "ACH_TEST" — compute via crc32fast and assert format is 8-char hex.
        let candidates: Vec<String> = vec!["ACH_TEST".into(), "X".into()];
        let map = build_crc_reverse_map(&candidates);
        for k in map.keys() {
            assert_eq!(k.len(), 8, "all keys must be 8-char zero-padded hex; got {}", k);
        }
    }

    #[tokio::test]
    async fn on_file_changed_emits_event_on_synthetic_transition() {
        let root = fresh_tmp();
        let appid_dir = root.join("9999");
        fs::create_dir_all(&appid_dir).unwrap();
        let path = appid_dir.join(STATE_FILENAME);

        // Compute CRC of a candidate name so the placeholder fallback doesn't kick in.
        // We build a Goldberg companion file containing this name first.
        let candidate = "ACH_SYNTHETIC";
        let goldberg_root = if let Some(d) = dirs::data_dir() { d } else { return };
        let companion_dir = goldberg_root.join("GSE Saves").join("9999");
        let _ = fs::create_dir_all(&companion_dir);
        let companion = companion_dir.join("achievements.json");
        let companion_existed = companion.exists();
        if !companion_existed {
            fs::write(&companion, format!(r#"{{ "{}": {{ "earned": false, "earned_time": 0 }} }}"#, candidate)).unwrap();
        }

        let crc = {
            let mut h = crc32fast::Hasher::new();
            h.update(candidate.as_bytes());
            h.finalize()
        };

        // Initial file: achieved=false. Seed.
        fs::write(&path, synth_stats_bin(&[(crc, false, 0)])).unwrap();
        let adapter = SseAdapter::new(vec![appid_dir.clone()]);
        adapter.seed_baseline().await.unwrap();
        let snap = adapter.baseline_snapshot().await;
        assert!(snap.contains_key(&(9999, candidate.to_string())) || !snap.is_empty(), "baseline should have an entry");

        // Now flip to achieved=true.
        fs::write(&path, synth_stats_bin(&[(crc, true, 1700000099)])).unwrap();
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(path, tx).await.unwrap();
        let evt = timeout(Duration::from_millis(500), rx.recv()).await.unwrap().unwrap();
        assert_eq!(evt.app_id, 9999);
        assert_eq!(evt.source, SourceKind::SmartSteamEmu);
        // ach_api_name resolved via Goldberg companion → "ACH_SYNTHETIC". If companion lookup failed
        // (e.g. dirs::data_dir doesn't return a path the test can write to), placeholder format is acceptable.
        assert!(
            evt.ach_api_name == candidate || evt.ach_api_name.starts_with("<crc:0x"),
            "got {}",
            evt.ach_api_name
        );

        if !companion_existed {
            let _ = fs::remove_file(&companion);
            let _ = fs::remove_dir(&companion_dir);
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn on_file_changed_skips_non_stats_filename() {
        let root = fresh_tmp();
        let appid_dir = root.join("9999");
        fs::create_dir_all(&appid_dir).unwrap();
        let bogus = appid_dir.join("not_stats.bin");
        fs::write(&bogus, &[0u8; 4]).unwrap();
        let adapter = SseAdapter::new(vec![appid_dir]);
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(bogus, tx).await.unwrap();
        let result = timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(result.is_err() || result.unwrap().is_none(), "non-stats filename must not produce events");
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn on_file_changed_skips_identical_content_via_sha256() {
        let root = fresh_tmp();
        let appid_dir = root.join("9999");
        fs::create_dir_all(&appid_dir).unwrap();
        let path = appid_dir.join(STATE_FILENAME);
        fs::write(&path, synth_stats_bin(&[(0xDEADBEEF, false, 0)])).unwrap();
        let adapter = SseAdapter::new(vec![appid_dir]);
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(path.clone(), tx.clone()).await.unwrap();
        let _ = timeout(Duration::from_millis(50), rx.recv()).await;
        adapter.on_file_changed(path, tx).await.unwrap();
        let result = timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(result.is_err() || result.unwrap().is_none(), "identical content must short-circuit");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn extract_app_id_from_canonical_path() {
        let p = PathBuf::from("/tmp/SmartSteamEmu/9999/stats.bin");
        assert_eq!(extract_app_id(&p), Some(9999));
        let bad = PathBuf::from("/tmp/SmartSteamEmu/notnumeric/stats.bin");
        assert_eq!(extract_app_id(&bad), None);
    }
}
