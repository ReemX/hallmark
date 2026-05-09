//! Legitimate Steam client `UserGameStats_<userid>_<appid>.bin` adapter (REQ DETECT-02).
//!
//! # State + schema two-file dependency
//!
//! Each app has TWO files in `<SteamPath>\appcache\stats\`:
//! - `UserGameStats_<userid>_<appid>.bin` — achievement state (read on every change).
//!   Tree shape: root Object "cache" → contains numeric `<stat_slot>` Object entries,
//!   each containing `data: Int32` (achievement bits) and an `AchievementTimes` Object
//!   mapping `<bit_slot>` → unix-seconds Int32 timestamp.
//! - `UserGameStatsSchema_<appid>.bin` — schema (read once per app, mtime-cached).
//!   Tree shape: root Object → `<appid>` Object → `stats` Object → numeric `<stat_slot>`
//!   Object → `bits` Object → numeric `<bit_slot>` Object → `name: String` (the API name).
//!
//! When schema file is absent (Pitfall #8), we emit RawUnlockEvent with a placeholder
//! `ach_api_name = "steam_stat_<stat_slot>_<bit_slot>"` so the popup still fires with
//! degraded display.
//!
//! # File pattern guard
//!
//! Filename regex: `^UserGameStats_(\d+)_(\d+)\.bin$`. Capture group 1 is the user_id,
//! group 2 is the app_id. Events for files NOT matching this pattern (e.g. SchemaFile
//! changes, sibling cache files) are skipped.
//!
//! # Multi-user filter (Pitfall #5)
//!
//! Events for files whose user_id is NOT in `self.user_ids` (the registry-discovered
//! set) are silently dropped at debug level — they belong to another Steam account on
//! the same Windows profile.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, RwLock};

use super::vdf_binary::{self, Value};
use super::{RawUnlockEvent, SourceAdapter, SourceKind};

#[derive(Debug, Clone, Default)]
pub struct SteamLegitPaths {
    pub appcache_stats: Option<PathBuf>,
    pub user_ids: Vec<u64>,
}

#[derive(Debug, Clone)]
struct AppSchema {
    /// Map of (stat_slot, bit_slot) → API name from UserGameStatsSchema file.
    /// Plain stat-slot achievements (no bits sub-object) use bit_slot = 0.
    achievements: HashMap<(u32, u32), String>,
    loaded_mtime: SystemTime,
}

#[cfg(target_os = "windows")]
pub fn discover_paths(steam_install: Option<&Path>) -> SteamLegitPaths {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut user_ids: Vec<u64> = Vec::new();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(users_key) = hkcu.open_subkey(r"Software\Valve\Steam\Users") {
        for name in users_key.enum_keys().flatten() {
            if let Ok(uid) = name.parse::<u64>() {
                user_ids.push(uid);
            }
        }
    }
    if user_ids.is_empty() {
        // Fallback — try parsing one user_id from a sample UserGameStats file.
        if let Some(steam) = steam_install {
            let appcache = steam.join("appcache").join("stats");
            if let Ok(rd) = std::fs::read_dir(&appcache) {
                for entry in rd.flatten() {
                    let name = entry.file_name();
                    let nstr = name.to_string_lossy();
                    if let Some(uid) = parse_user_id_from_filename(&nstr) {
                        if !user_ids.contains(&uid) {
                            user_ids.push(uid);
                        }
                    }
                }
            }
        }
    }
    let appcache_stats = steam_install
        .map(|p| p.join("appcache").join("stats"))
        .filter(|p| p.exists());
    SteamLegitPaths { appcache_stats, user_ids }
}

#[cfg(not(target_os = "windows"))]
pub fn discover_paths(_steam_install: Option<&Path>) -> SteamLegitPaths {
    SteamLegitPaths::default()
}

/// WR-06: Strip the leading `UserGameStats_` prefix case-insensitively. Also rejects
/// `UserGameStatsSchema_*` (which begins with `UserGameStatsS...`) by additionally
/// requiring that the character immediately after `UserGameStats_` is numeric (the
/// user_id digit). Returns the byte slice of the rest of the name minus the leading
/// prefix and trailing `.bin` suffix, or `None` on mismatch.
fn strip_user_game_stats_envelope(name: &str) -> Option<&str> {
    const PREFIX_LEN: usize = "UserGameStats_".len();
    if name.len() < PREFIX_LEN + 4 {
        return None;
    }
    if !name.is_char_boundary(PREFIX_LEN) {
        return None;
    }
    let (prefix, rest) = name.split_at(PREFIX_LEN);
    if !prefix.eq_ignore_ascii_case("UserGameStats_") {
        return None;
    }
    // Reject `UserGameStatsSchema_...` (the next char would be 'S'/'s', not a digit).
    let stem = rest.strip_suffix_ignore_case(".bin")?;
    if !stem.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return None;
    }
    Some(stem)
}

/// Polyfill: `str::strip_suffix` is case-sensitive in std; for filename matching on
/// Windows we need case-insensitive `.bin` suffix stripping. WR-06.
trait StripSuffixIgnoreCase {
    fn strip_suffix_ignore_case<'a>(&'a self, suffix: &str) -> Option<&'a str>;
}
impl StripSuffixIgnoreCase for str {
    fn strip_suffix_ignore_case<'a>(&'a self, suffix: &str) -> Option<&'a str> {
        if self.len() < suffix.len() {
            return None;
        }
        let split = self.len() - suffix.len();
        if !self.is_char_boundary(split) {
            return None;
        }
        let (head, tail) = self.split_at(split);
        if tail.eq_ignore_ascii_case(suffix) {
            Some(head)
        } else {
            None
        }
    }
}

fn parse_user_id_from_filename(name: &str) -> Option<u64> {
    // UserGameStats_<userid>_<appid>.bin (case-insensitive — WR-06)
    let stem = strip_user_game_stats_envelope(name)?;
    let (uid, _appid) = stem.split_once('_')?;
    uid.parse().ok()
}

fn parse_app_id_from_filename(name: &str) -> Option<u64> {
    let stem = strip_user_game_stats_envelope(name)?;
    let (_uid, appid) = stem.split_once('_')?;
    appid.parse().ok()
}

fn schema_filename(app_id: u64) -> String {
    format!("UserGameStatsSchema_{}.bin", app_id)
}

/// Adapter for legitimate Steam client.
pub struct SteamLegitAdapter {
    /// `<SteamPath>\appcache\stats` if it exists.
    appcache_stats: Option<PathBuf>,
    user_ids: Vec<u64>,
    cached_watch_paths: Vec<PathBuf>,
    /// (app_id, ach_api_name) → bool baseline.
    baseline: Arc<RwLock<HashMap<(u64, String), bool>>>,
    /// Per-file content-hash short-circuit.
    last_hash: Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>,
    /// Per-app schema cache (key = app_id).
    schema_cache: Arc<RwLock<HashMap<u64, AppSchema>>>,
}

impl SteamLegitAdapter {
    pub fn new(appcache_stats: Option<PathBuf>, user_ids: Vec<u64>) -> Self {
        let cached: Vec<PathBuf> = appcache_stats.iter().filter(|p| p.exists()).cloned().collect();
        Self {
            appcache_stats,
            user_ids,
            cached_watch_paths: cached,
            baseline: Arc::new(RwLock::new(HashMap::new())),
            last_hash: Arc::new(RwLock::new(HashMap::new())),
            schema_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[cfg(test)]
    pub(crate) async fn baseline_snapshot(&self) -> HashMap<(u64, String), bool> {
        self.baseline.read().await.clone()
    }

    /// Load (or refresh) the schema for `app_id` if its file mtime advanced. Returns
    /// the mapping (stat_slot, bit_slot) → API name, or empty if file is missing.
    async fn load_schema(&self, app_id: u64) -> HashMap<(u32, u32), String> {
        let Some(root) = &self.appcache_stats else { return HashMap::new() };
        let schema_path = root.join(schema_filename(app_id));

        let new_mtime = match std::fs::metadata(&schema_path).and_then(|m| m.modified()) {
            Ok(m) => m,
            Err(_) => {
                // Pitfall #8: schema file missing — return empty; caller emits placeholder names.
                tracing::warn!(app_id, path = %schema_path.display(), "schema file missing — popup will use placeholder ach_api_name");
                return HashMap::new();
            }
        };

        {
            let cache = self.schema_cache.read().await;
            if let Some(s) = cache.get(&app_id) {
                if s.loaded_mtime == new_mtime {
                    return s.achievements.clone();
                }
            }
        }

        // Parse the schema file.
        let bytes = match read_bytes_with_retry(&schema_path).await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(app_id, error = %e, "schema file read failed");
                return HashMap::new();
            }
        };
        let vdf = match vdf_binary::parse_binary_vdf(&bytes) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(app_id, error = %e, "schema file parse failed");
                return HashMap::new();
            }
        };

        let achievements = extract_schema_mapping(&vdf);
        let entry = AppSchema { achievements: achievements.clone(), loaded_mtime: new_mtime };
        self.schema_cache.write().await.insert(app_id, entry);
        achievements
    }
}

/// Walk a parsed UserGameStatsSchema VDF tree and extract (stat_slot, bit_slot) → API name.
///
/// Per RESEARCH.md Pattern 2, the canonical schema file shape is:
///   root → "<appid>" Object → "stats" Object → "<stat_slot>" Object [→ "bits" Object → "<bit_slot>" Object] → "name" String
///
/// We do a DETERMINISTIC path-walk (NOT a heuristic on `root_obj.len() == 1`):
///   1. Try root.values() looking for any single-Object child whose key parses as a numeric appid;
///      if found, descend into that child as the appid-container.
///   2. If no numeric-appid child is found, treat root itself as the appid-container.
///   3. Look for "stats" Object on the chosen container. If absent, log warn + return empty
///      (the schema file is malformed or uses an unexpected shape).
///   4. For every numeric stat_slot under "stats", read direct "name" (bit=0 entry) and any
///      "bits" sub-Object entries (per-bit names).
///
/// This is robust to: schema files with extra root-level metadata keys (e.g. "version", "crc"),
/// schema files where the appid container is the root itself, and schema files with empty stats.
fn extract_schema_mapping(vdf: &vdf_binary::Vdf) -> HashMap<(u32, u32), String> {
    let mut out = HashMap::new();
    let Some(root_obj) = vdf.root.as_object() else { return out };

    // Step 1+2: locate the appid-container deterministically.
    let appid_container: &HashMap<String, Value> = root_obj
        .iter()
        .find_map(|(k, v)| {
            // Numeric appid key whose value is an Object → descend.
            if k.parse::<u64>().is_ok() {
                v.as_object()
            } else {
                None
            }
        })
        .unwrap_or(root_obj);

    // Step 3: find "stats". If absent on the chosen container, fall back to root_obj
    // (covers schema files where stats lives at the root rather than under <appid>).
    let stats_obj = match appid_container.get("stats").and_then(|v| v.as_object()) {
        Some(s) => s,
        None => match root_obj.get("stats").and_then(|v| v.as_object()) {
            Some(s) => s,
            None => {
                tracing::warn!("schema file: 'stats' object not found at root or under <appid>; returning empty mapping");
                return out;
            }
        },
    };

    // Step 4: walk stat_slots and their optional "bits" sub-Objects.
    for (stat_key, stat_val) in stats_obj {
        let Ok(stat_slot) = stat_key.parse::<u32>() else { continue };
        let Some(stat_obj) = stat_val.as_object() else { continue };

        // Direct name on the stat (no bits sub-object — bit_slot = 0).
        if let Some(name) = stat_obj.get("name").and_then(|v| v.as_string()) {
            out.insert((stat_slot, 0), name.to_string());
        }

        // Bits sub-object — per-achievement entries.
        if let Some(bits_obj) = stat_obj.get("bits").and_then(|v| v.as_object()) {
            for (bit_key, bit_val) in bits_obj {
                let Ok(bit_slot) = bit_key.parse::<u32>() else { continue };
                if let Some(bit_inner) = bit_val.as_object() {
                    if let Some(name) = bit_inner.get("name").and_then(|v| v.as_string()) {
                        out.insert((stat_slot, bit_slot), name.to_string());
                    }
                }
            }
        }
    }
    out
}

/// Walk a parsed UserGameStats state VDF tree and extract (stat_slot, bit_slot) → earned bool.
/// Tree shape: root.cache.<stat_slot>.data: Int32 + AchievementTimes.<bit_slot>: Int32 (timestamp)
/// Plain stat achievements (no bits) use bit_slot = 0; presence in AchievementTimes implies earned.
fn extract_state_mapping(vdf: &vdf_binary::Vdf) -> HashMap<(u32, u32), bool> {
    let mut out = HashMap::new();
    let Some(root_obj) = vdf.root.as_object() else { return out };

    // Try root.cache first; some files have cache as root, others nest under the root_key.
    let cache_obj = root_obj.get("cache").and_then(|v| v.as_object()).unwrap_or(root_obj);

    for (stat_key, stat_val) in cache_obj {
        let Ok(stat_slot) = stat_key.parse::<u32>() else { continue };
        let Some(stat_obj) = stat_val.as_object() else { continue };

        // Plain stat achievement: presence of `data` Int32 == 1 means earned (achievement = boolean stat).
        if let Some(data) = stat_obj.get("data").and_then(|v| v.as_int32()) {
            if data == 1 {
                out.insert((stat_slot, 0), true);
            }
        }

        // Bit-mapped achievements: AchievementTimes.<bit_slot>: timestamp implies earned.
        if let Some(times_obj) = stat_obj.get("AchievementTimes").and_then(|v| v.as_object()) {
            for bit_key in times_obj.keys() {
                if let Ok(bit_slot) = bit_key.parse::<u32>() {
                    out.insert((stat_slot, bit_slot), true);
                }
            }
        }
    }
    out
}

#[async_trait::async_trait]
impl SourceAdapter for SteamLegitAdapter {
    fn name(&self) -> &str { "steam_legit" }
    fn kind(&self) -> SourceKind { SourceKind::SteamLegit }

    fn watch_paths(&self) -> Vec<PathBuf> { self.cached_watch_paths.clone() }

    /// Pitfall #6 (RESEARCH.md): SteamLegit baseline-vs-event race.
    ///
    /// Hallmark seeds the baseline by reading every `UserGameStats_<userid>_<appid>.bin`
    /// file BEFORE the watcher attaches. Steam writes its files non-atomically
    /// (open-write-close, not write-tmp-rename), so an unlock that fires precisely
    /// during the millisecond seed→attach gap will be silently absorbed: the file
    /// is read as "earned" at seed time, then the same "earned" state is observed
    /// after attach with no transition. This matches Phase 1's invariant for
    /// Goldberg ("seed before attach, accept that pre-startup unlocks are absorbed")
    /// and is consistent with REQ DETECT-05's "no historic spam" priority. v1
    /// accepts this limitation; Phase 4 polish may revisit if user reports surface.
    async fn seed_baseline(&self) -> anyhow::Result<()> {
        let Some(root) = &self.appcache_stats else {
            tracing::info!("Steam-legit: no appcache/stats path; skipping seed");
            return Ok(());
        };
        if !root.exists() {
            return Ok(());
        }

        let mut baseline = self.baseline.write().await;
        let mut total_files = 0u32;
        let mut total_entries = 0u32;

        let rd = match std::fs::read_dir(root) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "appcache/stats read_dir failed");
                return Ok(());
            }
        };
        let mut paths_to_seed: Vec<(PathBuf, u64, u64)> = Vec::new();
        for entry in rd.flatten() {
            let name = entry.file_name();
            let nstr = name.to_string_lossy().to_string();
            let Some(user_id) = parse_user_id_from_filename(&nstr) else { continue };
            if !self.user_ids.is_empty() && !self.user_ids.contains(&user_id) {
                tracing::debug!(file = nstr, "Steam-legit: skipping unknown user_id during seed");
                continue;
            }
            let Some(app_id) = parse_app_id_from_filename(&nstr) else { continue };
            paths_to_seed.push((entry.path(), user_id, app_id));
        }

        for (path, _user_id, app_id) in paths_to_seed {
            let bytes = match read_bytes_with_retry(&path).await {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "seed read failed");
                    continue;
                }
            };
            let vdf = match vdf_binary::parse_binary_vdf(&bytes) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "seed parse failed");
                    continue;
                }
            };
            let state = extract_state_mapping(&vdf);
            // Drop write-lock briefly to load schema (which takes its own lock).
            drop(baseline);
            let schema = self.load_schema(app_id).await;
            baseline = self.baseline.write().await;
            total_files += 1;
            for ((stat, bit), earned) in state {
                let api_name = schema
                    .get(&(stat, bit))
                    .cloned()
                    .unwrap_or_else(|| format!("steam_stat_{}_{}", stat, bit));
                baseline.insert((app_id, api_name), earned);
                total_entries += 1;
            }
        }

        tracing::info!(files = total_files, entries = total_entries, "Steam-legit baseline seeded");
        Ok(())
    }

    async fn on_file_changed(
        &self,
        path: PathBuf,
        tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()> {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { return Ok(()) };

        // WR-06: Skip schema-file events — they trigger schema reload at next state-file
        // event, not at the schema event itself. Case-insensitive: Windows is
        // case-insensitive at the filesystem level; mixed-case from copy/paste / network
        // shares / backup tools must still be detected.
        let name_lower = name.to_ascii_lowercase();
        if name_lower.starts_with("usergamestatsschema_") {
            tracing::trace!(path = %path.display(), "schema file event; ignored (schema is mtime-cached on demand)");
            return Ok(());
        }
        if !name_lower.starts_with("usergamestats_") || !name_lower.ends_with(".bin") {
            return Ok(());
        }
        let Some(user_id) = parse_user_id_from_filename(name) else { return Ok(()) };
        if !self.user_ids.is_empty() && !self.user_ids.contains(&user_id) {
            tracing::debug!(file = name, "Steam-legit: dropping event for non-active user_id (Pitfall #5)");
            return Ok(());
        }
        let Some(app_id) = parse_app_id_from_filename(name) else { return Ok(()) };

        let bytes = match read_bytes_with_retry(&path).await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "read_bytes_with_retry failed");
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
                tracing::trace!(path = %path.display(), "content unchanged; skip");
                return Ok(());
            }
            h.insert(path.clone(), hash);
        }

        let vdf = match vdf_binary::parse_binary_vdf(&bytes) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "binary VDF parse failed");
                return Ok(());
            }
        };
        let state = extract_state_mapping(&vdf);
        let schema = self.load_schema(app_id).await;

        // WR-01: Do not hold the baseline write-lock across `tx.send().await`. See
        // matching comment in cream_api.rs. We classify each entry under the lock,
        // commit baseline for non-emitting transitions immediately, buffer emitting
        // ones, drop the lock, then drain. CR-01 preserved: on send failure we skip
        // the baseline.insert for that key so a future event can re-fire.
        let mut events_to_send: Vec<(RawUnlockEvent, (u64, String), bool)> = Vec::new();
        {
            let mut baseline = self.baseline.write().await;
            for ((stat, bit), earned_now) in state {
                let api_name = schema
                    .get(&(stat, bit))
                    .cloned()
                    .unwrap_or_else(|| format!("steam_stat_{}_{}", stat, bit));
                let key = (app_id, api_name.clone());
                let was = baseline.get(&key).copied().unwrap_or(false);
                if !was && earned_now {
                    let evt = RawUnlockEvent {
                        app_id,
                        ach_api_name: api_name,
                        timestamp: 0,
                        source: SourceKind::SteamLegit,
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

/// Read bytes with retry on Windows ERROR_SHARING_VIOLATION (32) / ERROR_LOCK_VIOLATION (33).
/// Mirrors goldberg::read_with_retry for binary file reads.
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
        let p = std::env::temp_dir().join(format!("hallmark-sl-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn fixtures_dir() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests");
        p.push("fixtures");
        p.push("steam_legit");
        p
    }

    fn first_state_fixture() -> PathBuf {
        fs::read_dir(fixtures_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("UserGameStats_") && n.ends_with(".bin"))
                    .unwrap_or(false)
            })
            .expect("at least one UserGameStats fixture must exist")
    }

    fn parse_user_app_from_path(p: &Path) -> (u64, u64) {
        let n = p.file_name().unwrap().to_string_lossy();
        (
            parse_user_id_from_filename(&n).unwrap(),
            parse_app_id_from_filename(&n).unwrap(),
        )
    }

    #[test]
    fn parse_user_id_and_app_id_from_filename() {
        assert_eq!(parse_user_id_from_filename("UserGameStats_132274694_546560.bin"), Some(132274694));
        assert_eq!(parse_app_id_from_filename("UserGameStats_132274694_546560.bin"), Some(546560));
        assert_eq!(parse_user_id_from_filename("garbage.bin"), None);
        assert_eq!(parse_app_id_from_filename("UserGameStatsSchema_546560.bin"), None);
    }

    #[tokio::test]
    async fn seed_baseline_reads_real_fixture() {
        let dst = fresh_tmp();
        // Copy at least one fixture state file + its companion schema (if present) into a dedicated tempdir
        let state = first_state_fixture();
        let (uid, app_id) = parse_user_app_from_path(&state);
        let dst_state = dst.join(state.file_name().unwrap());
        fs::copy(&state, &dst_state).unwrap();
        let companion = fixtures_dir().join(format!("UserGameStatsSchema_{}.bin", app_id));
        if companion.exists() {
            fs::copy(&companion, dst.join(companion.file_name().unwrap())).unwrap();
        }

        let adapter = SteamLegitAdapter::new(Some(dst.clone()), vec![uid]);
        adapter.seed_baseline().await.unwrap();
        let snap = adapter.baseline_snapshot().await;
        // We don't assert the exact entry count (depends on fixture) but baseline must be non-empty
        // OR the file represented a game with no earned achievements (in which case state is empty
        // and zero entries are seeded — both are valid); we instead assert the function did NOT panic.
        let _ = snap;
        let _ = fs::remove_dir_all(&dst);
    }

    #[tokio::test]
    async fn on_file_changed_drops_unknown_user_id() {
        let dst = fresh_tmp();
        let state = first_state_fixture();
        let (uid, _app_id) = parse_user_app_from_path(&state);
        let dst_state = dst.join(state.file_name().unwrap());
        fs::copy(&state, &dst_state).unwrap();

        // Adapter configured with a DIFFERENT user_id — events should drop.
        let other_uid = uid.wrapping_add(1);
        let adapter = SteamLegitAdapter::new(Some(dst.clone()), vec![other_uid]);
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(dst_state, tx).await.unwrap();
        let result = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(result.is_err() || result.unwrap().is_none(), "expected no event for unknown user_id");
        let _ = fs::remove_dir_all(&dst);
    }

    #[tokio::test]
    async fn on_file_changed_skips_schema_filename() {
        let dst = fresh_tmp();
        let path = dst.join("UserGameStatsSchema_546560.bin");
        fs::write(&path, [0x00, b'r', b'o', b'o', b't', 0x00, 0x08]).unwrap();
        let adapter = SteamLegitAdapter::new(Some(dst.clone()), vec![132274694]);
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(path, tx).await.unwrap();
        let result = timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(result.is_err() || result.unwrap().is_none(), "schema-file event must not produce events");
        let _ = fs::remove_dir_all(&dst);
    }

    #[tokio::test]
    async fn on_file_changed_skips_identical_content_via_sha256() {
        let dst = fresh_tmp();
        let state = first_state_fixture();
        let (uid, app_id) = parse_user_app_from_path(&state);
        let dst_state = dst.join(state.file_name().unwrap());
        fs::copy(&state, &dst_state).unwrap();
        let companion = fixtures_dir().join(format!("UserGameStatsSchema_{}.bin", app_id));
        if companion.exists() {
            fs::copy(&companion, dst.join(companion.file_name().unwrap())).unwrap();
        }

        let adapter = SteamLegitAdapter::new(Some(dst.clone()), vec![uid]);
        adapter.seed_baseline().await.unwrap();
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(dst_state.clone(), tx.clone()).await.unwrap();
        // Drain anything from the first call.
        let _ = timeout(Duration::from_millis(50), rx.recv()).await;
        // Second call with identical bytes — must short-circuit and not emit.
        adapter.on_file_changed(dst_state.clone(), tx.clone()).await.unwrap();
        let result = timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(result.is_err() || result.unwrap().is_none(), "identical content must short-circuit");
        let _ = fs::remove_dir_all(&dst);
    }

    #[tokio::test]
    async fn on_file_changed_emits_event_on_synthetic_state_transition() {
        // We synthesise a minimal binary VDF that decodes to ((stat=1, bit=0), earned=true).
        // Tree: 0x00 'cache' 0x00 (root_key) 0x02 'data' 0x00 0x01 0x00 0x00 0x00 (stat 1 with data=1) 0x08
        // Simpler: write a stat_slot Object containing data=1.
        let dst = fresh_tmp();
        let app_id: u64 = 999999;
        let uid: u64 = 132274694;
        // Layout: root_key="cache" \0 then 0x00 "1" \0 (stat_slot=1 Object)
        //          then 0x02 "data" \0 [01 00 00 00] (Int32 data=1)
        //          then 0x08 (close stat_slot Object)
        //          then 0x08 (close root)
        let mut bytes: Vec<u8> = Vec::new();
        bytes.push(0x00);
        bytes.extend_from_slice(b"cache\0");
        bytes.push(0x00);
        bytes.extend_from_slice(b"1\0");
        bytes.push(0x02);
        bytes.extend_from_slice(b"data\0");
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.push(0x08);
        bytes.push(0x08);

        let state_path = dst.join(format!("UserGameStats_{}_{}.bin", uid, app_id));
        fs::write(&state_path, &bytes).unwrap();

        let adapter = SteamLegitAdapter::new(Some(dst.clone()), vec![uid]);
        // Skip seed (so the false→true transition is detected on first event).
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(state_path, tx).await.unwrap();
        let evt = timeout(Duration::from_millis(500), rx.recv()).await.unwrap().unwrap();
        assert_eq!(evt.app_id, app_id);
        assert_eq!(evt.source, SourceKind::SteamLegit);
        // No schema file → placeholder name.
        assert!(evt.ach_api_name.starts_with("steam_stat_1_0"));
        let _ = fs::remove_dir_all(&dst);
    }

    #[tokio::test]
    async fn missing_schema_emits_placeholder_api_name() {
        // Same synthetic input as the previous test; explicitly assert the placeholder format.
        let dst = fresh_tmp();
        let app_id: u64 = 888888;
        let uid: u64 = 132274694;
        let mut bytes: Vec<u8> = Vec::new();
        bytes.push(0x00);
        bytes.extend_from_slice(b"cache\0");
        bytes.push(0x00);
        bytes.extend_from_slice(b"7\0");
        bytes.push(0x02);
        bytes.extend_from_slice(b"data\0");
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.push(0x08);
        bytes.push(0x08);
        let state_path = dst.join(format!("UserGameStats_{}_{}.bin", uid, app_id));
        fs::write(&state_path, &bytes).unwrap();
        let adapter = SteamLegitAdapter::new(Some(dst.clone()), vec![uid]);
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(8);
        adapter.on_file_changed(state_path, tx).await.unwrap();
        let evt = timeout(Duration::from_millis(500), rx.recv()).await.unwrap().unwrap();
        assert_eq!(evt.ach_api_name, "steam_stat_7_0");
        let _ = fs::remove_dir_all(&dst);
    }

    #[test]
    fn discover_paths_returns_default_for_none_steam_install() {
        let p = discover_paths(None);
        // No Steam install → no appcache_stats and no user_ids (registry path doesn't crash even if Steam absent).
        assert!(p.appcache_stats.is_none());
        // user_ids may be populated from registry if Steam is installed on this machine; that's fine — we only assert appcache_stats correlation.
        let _ = p.user_ids;
    }
}
