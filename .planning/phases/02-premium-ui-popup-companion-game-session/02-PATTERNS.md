# Phase 2: Premium UI — Popup, Companion & Game Session - Pattern Map

**Mapped:** 2026-05-08
**Files analyzed:** ~30 new + 3 modified
**Analogs found:** 22 (Phase 1 source) + 8 (no analog — frontend net-new)
**Phase 1 source files read:** `src-tauri/src/lib.rs`, `src-tauri/src/store/mod.rs`, `src-tauri/src/store/queries.rs`, `src-tauri/src/store/migrations/001_initial.sql`, `src-tauri/src/watcher/mod.rs`, `src-tauri/src/watcher/dedup.rs`, `src-tauri/src/sources/mod.rs`, `src-tauri/src/sources/goldberg.rs`, `src-tauri/src/paths.rs`, `src-tauri/src/bin/hallmark-cli.rs`, `src-tauri/src/main.rs`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `Cargo.toml`

---

## File Classification

### Rust backend — new files

| New File | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|----------------|---------------|
| `src-tauri/src/ui.rs` | window-builder module | request-response (one-shot setup) | `src-tauri/src/lib.rs` (Tauri builder + setup hook) | role-match (no existing window-builder; lib.rs is closest by Tauri-API surface) |
| `src-tauri/src/popup_queue.rs` | event-driven consumer task | streaming (mpsc drain → IPC emit) | `src-tauri/src/watcher/mod.rs::run_pipeline` | exact (both are tokio drain loops over `mpsc::Receiver<RawUnlockEvent>`) |
| `src-tauri/src/audio.rs` | service (audio dispatcher) | request-response (sync `play()` call) | `src-tauri/src/sources/goldberg.rs::GoldbergAdapter` (struct-with-Arc-state pattern) | role-match (struct-with-pre-loaded-state shape; different domain) |
| `src-tauri/src/game_detect/mod.rs` | event-driven task (orchestrator) | event-driven (poll + emit start/stop) | `src-tauri/src/watcher/mod.rs::run_watcher` | role-match (long-lived tokio task, dispatches downstream) |
| `src-tauri/src/game_detect/process_scan.rs` | service (sysinfo wrapper) | batch (refresh + scan) | `src-tauri/src/paths.rs::appmanifest_lookup` | partial-match (filesystem scan + parse, not process scan; appid-resolution logic shared) |
| `src-tauri/src/game_detect/steam_state.rs` | utility (VDF parser) | file-I/O (read VDF) | `src-tauri/src/paths.rs::parse_libraryfolders_text` | exact (both parse Steam VDF text via keyvalues-parser) |
| `src-tauri/src/schema/mod.rs` | service (resolution chain) | request-response | `src-tauri/src/store/queries.rs` (typed helpers calling store) | role-match (orchestrates lookups; not a query helper itself) |
| `src-tauri/src/schema/cache.rs` | model + queries (SQLite layer) | CRUD | `src-tauri/src/store/queries.rs` | exact (typed query helpers calling `with_conn`) |
| `src-tauri/src/schema/appcache.rs` | utility (file parser) | file-I/O | `src-tauri/src/paths.rs::appmanifest_lookup` | role-match (Steam local file → typed data) |
| `src-tauri/src/schema/steam_api.rs` | service (HTTP client) | request-response (HTTP) | none in Phase 1 (no HTTP yet) | no analog |
| `src-tauri/src/monitor.rs` | utility (Win32 wrapper) | request-response (sync) | `src-tauri/src/paths.rs::read_steam_install` | role-match (both `#[cfg(target_os = "windows")]` Win32-API wrappers) |
| `src-tauri/src/store/migrations/002_schema_cache.sql` | migration | DDL | `src-tauri/src/store/migrations/001_initial.sql` | exact |

### Rust backend — modified files

| Modified File | Role | Data Flow | Existing Pattern Source | Match Quality |
|---------------|------|-----------|-------------------------|---------------|
| `src-tauri/src/lib.rs` | app entry / setup orchestrator | event-driven (setup spawns) | `src-tauri/src/lib.rs::run` (extension point already in place) | self-extending (Phase 1 left the seam explicitly) |
| `src-tauri/src/store/mod.rs` | persistence handle | CRUD | `src-tauri/src/store/mod.rs::open` | self-extending (add 002 migration via `include_str!` + execute_batch chain) |
| `src-tauri/src/store/queries.rs` | typed query helpers | CRUD | `src-tauri/src/store/queries.rs::create_session` | exact (add new helpers in same file/style) |
| `src-tauri/Cargo.toml` | manifest | config | existing manifest | exact |
| `src-tauri/tauri.conf.json` | Tauri config | config | existing config | exact |
| `Cargo.toml` (workspace) | workspace manifest | config | existing manifest | exact |

### Frontend — net-new files (no Phase 1 analog)

| New File | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|----------------|---------------|
| `package.json` (repo root) | manifest | config | none | no analog |
| `vite.config.ts` | config | config | none | no analog |
| `tsconfig.json` | config | config | none | no analog |
| `popup.html` | entry | static | none | no analog |
| `index.html` (companion) | entry | static | `dist/index.html` (placeholder) | partial (placeholder only — Phase 2 replaces) |
| `src/main-popup.tsx` | entry | event-driven (listen → state) | none | no analog |
| `src/main-companion.tsx` | entry | event-driven | none | no analog |
| `src/components/PopupCard.tsx` | component | request-response (props → render) | none | no analog |
| `src/components/AchievementRow.tsx` | component | request-response | none | no analog |
| `src/components/FilterBar.tsx` | component | request-response | none | no analog |
| `src/components/SortToggle.tsx` | component | request-response | none | no analog |
| `src/components/CompanionHeader.tsx` | component | request-response | none | no analog |
| `src/components/SkeletonRow.tsx` | component | request-response | none | no analog |
| `src/components/EmptyState.tsx` | component | request-response | none | no analog |
| `src/hooks/usePopupListener.ts` | hook | event-driven | none | no analog |
| `src/hooks/useGameSession.ts` | hook | event-driven | none | no analog |
| `src/styles/popup.css` | stylesheet | static | none | no analog |
| `src/styles/companion.css` | stylesheet | static | none | no analog |
| `src/types.ts` | types | static | `src-tauri/src/sources/mod.rs::RawUnlockEvent` | partial (TS mirror of Rust struct shapes via serde) |

### Capabilities + assets — net-new

| New File | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|----------------|---------------|
| `src-tauri/capabilities/popup.json` | config | config | none | no analog |
| `src-tauri/capabilities/companion.json` | config | config | none | no analog |
| `assets/sfx/popup-standard.wav` | static asset | static | none | no analog |
| `assets/sfx/popup-rare.wav` | static asset | static | none | no analog |
| `assets/sfx/popup-100pct.wav` | static asset | static | none | no analog |
| `assets/icons/placeholder.png` | static asset | static | `src-tauri/icons/icon.ico` (placeholder only) | partial |

---

## Pattern Assignments

### `src-tauri/src/popup_queue.rs` (event-driven consumer task)

**Analog:** `src-tauri/src/watcher/mod.rs::run_pipeline` (lines 350-409)

**Why this is the closest match:** `run_pipeline` is the existing template for "drain a `mpsc::Receiver<RawUnlockEvent>`, do per-event work with shared state, optionally forward to a downstream sink." Phase 2's popup_queue replaces the persistence + sink-forward step with enrichment + IPC emit + audio dispatch + sleep-driven timing.

**Imports pattern** (from `watcher/mod.rs:24-32`):
```rust
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use crate::sources::{RawUnlockEvent, SourceAdapter};
```
Phase 2 adapts: drop `SourceAdapter` (not consumed here), add `tauri::{AppHandle, Manager, Emitter}`, add `crate::audio::AudioDispatcher`, `crate::schema::SchemaCache`, `crate::monitor`.

**Async drain loop pattern** (from `watcher/mod.rs:359-405` — `run_pipeline`):
```rust
pub async fn run_pipeline(
    mut raw_rx: mpsc::Receiver<RawUnlockEvent>,
    store: Arc<SqliteStore>,
    session_id: String,
    sink: mpsc::Sender<RawUnlockEvent>,
    dedup_ttl: Duration,
) -> anyhow::Result<()> {
    let dedup = Arc::new(TokioMutex::new(CrossSourceDedup::new(dedup_ttl)));

    while let Some(evt) = raw_rx.recv().await {
        // ...per-event work...
        if sink.send(evt).await.is_err() {
            tracing::debug!("downstream sink closed; pipeline draining");
        }
    }

    tracing::info!("run_pipeline shutting down (raw_rx closed)");
    Ok(())
}
```
**Copy verbatim:** the `while let Some(evt) = rx.recv().await` shape, the `tracing::info!` shutdown line, the `pub async fn` signature taking `mut rx` + supporting handles by value/clone.

**Backpressure/dedup precedent** (from `watcher/mod.rs:360-394`): Phase 2 uses `sink.len()` for adaptive compression (D-10) — the parallel of Phase 1's dedup check. Both are short, synchronous decisions made before the per-event work block.

**Tracing pattern** (from `watcher/mod.rs:396-401`):
```rust
tracing::info!(
    app_id = evt.app_id,
    ach = %evt.ach_api_name,
    source = %evt.source,
    "UNLOCK"
);
```
Phase 2 emits a `"POPUP_FIRED"` log with the same field-key style.

---

### `src-tauri/src/audio.rs` (service)

**Analog:** `src-tauri/src/sources/goldberg.rs::GoldbergAdapter` (lines 56-92)

**Why this is the closest match:** Both are services constructed once, hold pre-loaded state in `Arc`, and expose a small synchronous API. Goldberg's pattern of "pre-resolve at construction; never re-stat on hot path" maps directly to "pre-decode SFX bytes at construction; clone bytes per play."

**Construction pattern with pre-loaded state** (from `goldberg.rs:76-92`):
```rust
pub fn new(roots: Vec<PathBuf>, redirect_map: HashMap<PathBuf, u64>) -> Self {
    // WR-08: resolve `exists()` once at startup. After this, `watch_paths()`
    // returns a clone of `cached_watch_paths` without further filesystem syscalls.
    let mut cached: Vec<PathBuf> = roots.iter().filter(|p| p.exists()).cloned().collect();
    for redirect_parent in redirect_map.keys() {
        if redirect_parent.exists() && !cached.contains(redirect_parent) {
            cached.push(redirect_parent.clone());
        }
    }
    Self {
        roots,
        redirect_map,
        cached_watch_paths: cached,
        baseline: Arc::new(RwLock::new(HashMap::new())),
        last_hash: Arc::new(RwLock::new(HashMap::new())),
    }
}
```
**Pattern to copy:** `pub fn new(...) -> Self` resolves I/O once and caches; the struct fields are populated exhaustively before return; later API calls operate on cached state without re-doing the I/O. Phase 2: `AudioDispatcher::new()` opens the device sink and decodes SFX bytes once, holds them in `Arc<Vec<u8>>` so per-play clone is cheap.

**Struct-with-`Arc`-shared-state pattern** (from `goldberg.rs:67-69`):
```rust
baseline: Arc<RwLock<HashMap<(u64, String), bool>>>,
last_hash: Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>,
```
Phase 2: SFX bytes use `Arc<Vec<u8>>` (no interior mutability needed — pre-loaded once and never mutated).

**Error handling pattern** (from `goldberg.rs:175-187`):
```rust
let json = match read_with_retry(path).await {
    Ok(s) => s,
    Err(e) => {
        tracing::warn!(path = %path.display(), error = %e, "seed read failed; skip");
        continue;
    }
};
```
Phase 2 audio: `play()` returns `anyhow::Result<()>` and the caller logs at `tracing::warn!` with context fields — never panics on a single bad SFX play.

---

### `src-tauri/src/game_detect/mod.rs` (event-driven orchestrator task)

**Analog:** `src-tauri/src/watcher/mod.rs::run_watcher` (lines 40-141)

**Why this is the closest match:** Both are long-running tokio tasks that orchestrate periodic polling + downstream dispatch. `run_watcher` polls notify; game_detect polls sysinfo. Both bridge sync→async via tokio mpsc / `blocking_send`.

**Long-running task with periodic polling** — Phase 2 uses `tokio::time::interval` (3s for D-21):
```rust
// Phase 1 analog: run_watcher (lines 40-141) sets up a watcher then loops on recv().
// Phase 2: tokio::time::interval drives the same shape with a refresh-then-emit body.
pub async fn run(app: AppHandle, schema: SchemaCache) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(3));
    let mut sys = sysinfo::System::new_all();
    loop {
        interval.tick().await;
        // poll + scan + diff + emit
    }
}
```

**Tracing on lifecycle events** (from `watcher/mod.rs:74-118`):
```rust
tracing::info!(adapter = adapter.name(), path = %path.display(),
    "watching path recursively");
// ...
tracing::info!(
    adapters = adapters.len(),
    paths = path_owner.len(),
    "WatcherCore active"
);
```
Phase 2: `tracing::info!(app_id = ..., name = %..., "game-start emitted")` and `tracing::warn!(...)` for D-22 conflict resolution between Steam state and sysinfo.

**Conflict-resolution log pattern** (from `watcher/mod.rs:103-110`):
```rust
tracing::error!(
    adapter_a = adapters[*a_idx].name(),
    adapter_b = adapters[*b_idx].name(),
    path_a = %pa.display(),
    path_b = %pb.display(),
    "adapter watch paths overlap; events may be routed to multiple adapters"
);
```
Phase 2 uses identical structure for D-22: `tracing::warn!(steam_app_id = ..., sysinfo_app_id = ..., "Steam state vs sysinfo conflict; Steam wins")`.

---

### `src-tauri/src/game_detect/steam_state.rs` (VDF parser)

**Analog:** `src-tauri/src/paths.rs::parse_libraryfolders_text` (lines 216-265)

**Why this is the closest match:** Exact — both are pure functions that parse a Steam VDF text blob via the existing `keyvalues-parser 0.2` dependency. `loginusers.vdf` (Phase 2) has the same VDF shape as `libraryfolders.vdf` (Phase 1); the parsing logic is line-for-line transferable.

**VDF parsing pattern** (from `paths.rs:216-264`):
```rust
pub(crate) fn parse_libraryfolders_text(text: &str) -> Vec<PathBuf> {
    use keyvalues_parser::Vdf;

    let vdf = match Vdf::parse(text) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "libraryfolders.vdf parse failed");
            return Vec::new();
        }
    };

    // The top-level key is "libraryfolders" (post-2022) or "LibraryFolders" (legacy);
    // case-insensitive match.
    if !vdf.key.eq_ignore_ascii_case("libraryfolders") {
        tracing::warn!(top_key = %vdf.key, "libraryfolders.vdf has unexpected top-level key");
        return Vec::new();
    }

    let Some(obj) = vdf.value.get_obj() else {
        tracing::warn!("libraryfolders.vdf top-level value is not an object");
        return Vec::new();
    };

    let mut libs = Vec::new();
    for (entry_key, values) in obj.iter() {
        if entry_key.parse::<u32>().is_err() {
            continue;
        }
        for value in values.iter() {
            if let Some(s) = value.get_str() {
                libs.push(PathBuf::from(s));
            } else if let Some(sub_obj) = value.get_obj() {
                if let Some(path_values) = sub_obj.get("path") {
                    if let Some(path_value) = path_values.first() {
                        if let Some(path_str) = path_value.get_str() {
                            libs.push(PathBuf::from(path_str));
                        }
                    }
                }
            }
        }
    }
    libs
}
```
**Copy directly:** the `Vdf::parse` + `eq_ignore_ascii_case` top-key check + `get_obj()` + `for (key, values) in obj.iter()` traversal + `get_str()` / `get_obj()` value-shape branching. Phase 2 replaces "libraryfolders" with "users" / "MostRecent" + `RememberPassword` field per loginusers.vdf shape.

**Companion appmanifest pattern** (from `paths.rs:303-384` — for D-21 sysinfo fallback path):
```rust
pub(crate) fn appmanifest_lookup(library: &Path) -> HashMap<String, u64> {
    use keyvalues_parser::Vdf;
    // ... reads <library>/steamapps/appmanifest_*.acf, returns installdir → appid map
}
```
Phase 2's `process_scan.rs` reuses `appmanifest_lookup` directly (already `pub(crate)`) — extend visibility to `pub` for cross-module consumption, or keep local and re-implement. **Recommendation:** make `appmanifest_lookup` pub in `paths.rs` and import it from `game_detect::process_scan`.

---

### `src-tauri/src/monitor.rs` (Win32 wrapper)

**Analog:** `src-tauri/src/paths.rs::read_steam_install` (lines 136-162)

**Why this is the closest match:** Both are `#[cfg(target_os = "windows")]`-gated Win32 wrappers with non-Windows stubs. The `#[cfg(...)]` discipline + `Option<...>` return + tracing on read failure is identical.

**`#[cfg(target_os = "windows")]` + stub pattern** (from `paths.rs:136-171`):
```rust
#[cfg(target_os = "windows")]
fn read_steam_install() -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey(r"SOFTWARE\WOW6432Node\Valve\Steam") {
        if let Ok(p) = key.get_value::<String, _>("InstallPath") {
            let path = PathBuf::from(p);
            if path.exists() {
                return Some(path);
            }
            tracing::warn!(path = %path.display(), "Steam HKLM InstallPath does not exist on disk");
        }
    }
    // ...HKCU fallback...
    None
}

// Stub for non-Windows — Phase 1 is Windows-only but the cfg keeps the rest of the
// file compilable for hypothetical CI on Linux. (Per CLAUDE.md, Phase 1 is Win-only;
// this is just defensive scaffolding so `cargo check` stays green if anyone tries
// a Linux build.)
#[cfg(not(target_os = "windows"))]
fn read_steam_install() -> Option<PathBuf> {
    None
}
```
**Copy:** the dual-`#[cfg]` arms with identical signatures, the per-attempt `tracing::warn!` on fallible reads, the `Option<T>` return for "not found is normal."

**Phase 2 monitor.rs** (from RESEARCH.md Pattern 2 — applied via this analog):
- `hwnd_for_pid(pid: u32) -> Option<HWND>` — `#[cfg(target_os = "windows")]` real impl + non-Windows stub returning `None`.
- `monitor_rect_for_hwnd(hwnd: HWND) -> Option<(i32, i32, i32, i32)>` — same dual-cfg shape.
- `popup_position(...)` — pure math, no `#[cfg]` needed (works on any target).

**Public test helper pattern** (from `paths.rs:1063-1072`):
```rust
/// Public test entry: invoke `scan_local_save_redirects` against the given
/// libraries. Returns the resolved `Vec<GoldbergRedirect>`. Intended for
/// integration-test use only; production code should call `discover()`.
pub fn scan_local_save_redirects_pub_for_tests(libraries: &[PathBuf]) -> Vec<GoldbergRedirect> {
    scan_local_save_redirects(libraries)
}
```
**Apply for Phase 2:** if `popup_position()` needs to be exercised by external tests (likely), follow the same `_pub_for_tests` shim pattern rather than relaxing `pub(crate)` to `pub`.

---

### `src-tauri/src/schema/cache.rs` (typed query helpers)

**Analog:** `src-tauri/src/store/queries.rs` (full file, 141 lines)

**Why this is the closest match:** Exact. Phase 1's queries.rs is the established home for per-table typed query helpers that take `&Connection`. Phase 2 follows the same shape for `schema_cache`, `companion_prefs`, and `settings.completion_<appid>`.

**Typed insert helper pattern** (from `queries.rs:12-32`):
```rust
pub fn create_session(
    conn: &Connection,
    session_id: &str,
    app_id: Option<u64>,
) -> anyhow::Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    // IN-03: surface u64 → i64 overflow as an error rather than silent wrap.
    let app_id_i64 = match app_id {
        Some(a) => Some(i64::try_from(a)?),
        None => None,
    };
    conn.execute(
        "INSERT INTO sessions (session_id, app_id, started_at, ended_at)
         VALUES (?1, ?2, ?3, NULL)",
        params![session_id, app_id_i64, now],
    )?;
    Ok(())
}
```
**Copy directly:** the `pub fn name(conn: &Connection, ...)` signature, `let now = SystemTime::now() ...` epoch conversion, `i64::try_from(a)?` for u64-app_id overflow guard, `params![...]` macro, `conn.execute(SQL, params)` returning rows-changed.

**Typed update pattern** (from `queries.rs:34-44`):
```rust
pub fn end_session(conn: &Connection, session_id: &str) -> anyhow::Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    conn.execute(
        "UPDATE sessions SET ended_at = ?1 WHERE session_id = ?2 AND ended_at IS NULL",
        params![now, session_id],
    )?;
    Ok(())
}
```
**Phase 2 `mark_completion` mirrors this exactly:** `UPDATE settings SET value = ?1 WHERE key = ?2` or `INSERT OR REPLACE INTO settings (key, value) VALUES ('completion_<appid>', '1')`.

**Typed count/read pattern** (from `queries.rs:67-74`):
```rust
pub fn unlock_count_for_session(conn: &Connection, session_id: &str) -> anyhow::Result<i64> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM unlock_history WHERE session_id = ?1",
        params![session_id],
        |row| row.get(0),
    )?;
    Ok(n)
}
```
**Phase 2:** `count_schema_for_app(conn, app_id) -> i64`, `count_earned_for_app(conn, app_id, session_id) -> i64`. Used for D-11 100% trigger detection (`count_earned == count_schema` && `completion_<appid>` flag absent).

**Test fixtures pattern** (from `queries.rs:80-114`):
```rust
fn fresh_store() -> SqliteStore {
    SqliteStore::open_in_memory().unwrap()
}

#[test]
fn create_and_end_session_roundtrip() {
    let s = fresh_store();
    let conn = s.conn.lock().unwrap();
    create_session(&conn, "test-session-1", None).unwrap();
    // ...
}
```
**Apply:** Phase 2 unit tests use `SqliteStore::open_in_memory()` + a manual `s.conn.lock()` to exercise typed helpers. Note `s.conn` is `pub(super)` — tests in the same module tree can access it directly.

---

### `src-tauri/src/schema/mod.rs` (resolution chain orchestrator)

**Analog:** `src-tauri/src/sources/goldberg.rs::GoldbergAdapter::on_file_changed` (lines 237-313)

**Why this is the closest match:** Both orchestrate a sequence of fallible steps with early-exit on cache hit + `tracing::warn!` on each failed leg without aborting the chain. The "lookup chain" pattern (cache → appcache → Web API) parallels Goldberg's "read → hash-check → parse → diff → emit" sequence.

**Sequenced fallible-step pattern with early-exit + per-step tracing** (from `goldberg.rs:253-313`):
```rust
async fn on_file_changed(
    &self,
    path: PathBuf,
    tx: mpsc::Sender<RawUnlockEvent>,
) -> anyhow::Result<()> {
    // Filter early
    if path.file_name().and_then(|n| n.to_str()) != Some("achievements.json") {
        return Ok(());
    }
    let Some(app_id) = self.extract_app_id(&path) else {
        tracing::debug!(path = %path.display(),
            "could not resolve appid for event path; ignoring");
        return Ok(());
    };

    // Step 1: read
    let json = match read_with_retry(&path).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "read_with_retry failed");
            return Ok(());
        }
    };

    // Step 2: cache check (early-exit)
    let hash: [u8; 32] = Sha256::digest(json.as_bytes()).into();
    {
        let hashes = self.last_hash.read().await;
        if hashes.get(&path) == Some(&hash) {
            tracing::trace!(path = %path.display(), "content unchanged (hash match); skip");
            return Ok(());
        }
    }

    // Step 3: parse
    let state = match Self::parse_state(&json) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "state file parse failed");
            return Ok(());
        }
    };

    // Step 4: diff + emit
    // ...

    Ok(())
}
```
**Copy the early-exit + match-on-Result + per-step tracing pattern.** Phase 2's `SchemaCache::resolve(app_id)` flows: SQLite hit → return; appcache miss → log warn → continue; Web API miss → log warn → return cached-but-stale or empty.

**Read-with-retry on Windows transient errors** (from `goldberg.rs:328-358`):
```rust
async fn read_with_retry(path: &Path) -> anyhow::Result<String> {
    let mut last_err: Option<std::io::Error> = None;
    for _ in 0..3 {
        match std::fs::read_to_string(path) {
            Ok(s) => return Ok(s),
            Err(e)
                if e.kind() == std::io::ErrorKind::PermissionDenied
                    || matches!(
                        e.raw_os_error(),
                        Some(32) | Some(33)
                    ) =>
            {
                last_err = Some(e);
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(e) => return Err(e.into()),
        }
    }
    match last_err {
        Some(e) => Err(e.into()),
        None => Err(anyhow::anyhow!(
            "read_with_retry: 0 attempts configured; refusing"
        )),
    }
}
```
**Apply to Phase 2 appcache reads:** `librarycache/<appid>_*.jpg` may be open by Steam mid-write. Use the same retry shape.

---

### `src-tauri/src/store/migrations/002_schema_cache.sql` (DDL migration)

**Analog:** `src-tauri/src/store/migrations/001_initial.sql` (full file, 38 lines)

**Why this is the closest match:** Exact — same file conventions (header comment block, `IF NOT EXISTS`, `CREATE INDEX IF NOT EXISTS` for non-unique, `CREATE UNIQUE INDEX IF NOT EXISTS` for dedup constraints).

**Migration file structure pattern** (from `001_initial.sql`):
```sql
-- Phase 1 schema: unlock detection persistence.
-- Phase 2 will add schema_cache + icon_cache; Phase 3 may extend sessions.
-- This file is loaded via include_str! at compile time and applied idempotently
-- on every SqliteStore::open(). All statements use IF NOT EXISTS for restart safety.

CREATE TABLE IF NOT EXISTS unlock_history (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    app_id        INTEGER NOT NULL,
    ach_api_name  TEXT    NOT NULL,
    source        TEXT    NOT NULL,
    unlocked_at   INTEGER NOT NULL,
    -- WR-11: session_id is NOT NULL. SQLite treats NULL as distinct from NULL
    -- in UNIQUE INDEX, so allowing NULL silently disabled the dedup constraint
    -- whenever a bug elsewhere dropped the session_id. Production code always
    -- passes a session id (Plan 05); the schema now enforces that.
    session_id    TEXT    NOT NULL,
    notified      INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_unlock_session ON unlock_history(session_id);
CREATE INDEX IF NOT EXISTS idx_unlock_app     ON unlock_history(app_id, ach_api_name);
CREATE UNIQUE INDEX IF NOT EXISTS idx_unlock_dedup
    ON unlock_history(app_id, ach_api_name, session_id);
```
**Copy:** the header comment block (date, scope, idempotency note), the table-then-indexes block layout, the `IF NOT EXISTS` discipline on every DDL statement, the inline `-- WR-XX:` rationale comments for non-obvious schema decisions.

**Loader pattern** (from `store/mod.rs:16` and `store/mod.rs:26-32`):
```rust
const INITIAL_MIGRATION_SQL: &str = include_str!("migrations/001_initial.sql");
// ...
pub fn open(db_path: &Path) -> anyhow::Result<Self> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(INITIAL_MIGRATION_SQL)?;
    Ok(Self {
        conn: Mutex::new(conn),
    })
}
```
**Phase 2 extension:** add a second const `const PHASE2_MIGRATION_SQL: &str = include_str!("migrations/002_schema_cache.sql");` and a second `conn.execute_batch(...)` call in `open()`. Both are idempotent; order is 001 then 002.

---

### `src-tauri/src/lib.rs::run()` (modified — extend setup)

**Analog:** `src-tauri/src/lib.rs::run` (lines 60-79) — self-extending

**Phase 1's existing extension seam** (from `lib.rs:67-78`):
```rust
tauri::Builder::default()
    .setup(|_app| {
        // Plans 04 + 05 attach pipeline tasks here:
        //   tokio::spawn(watcher::run_watcher(...));
        //   tokio::spawn(cli::run_cli_sink(...));
        tracing::info!(
            "Tauri setup complete (no background tasks attached in Phase 1 scaffold)"
        );
        Ok(())
    })
    .run(tauri::generate_context!())
    .expect("Tauri runtime failed to start");
```
**Phase 2 fills in this seam.** `_app` becomes `app`; `app.handle().clone()` is the AppHandle for spawn closures. Order:
1. Open `SqliteStore` (Phase 1 `open()` now applies both 001 + 002 migrations).
2. Run `paths::discover()` (Phase 1 helper).
3. Build adapter list + create `RawUnlockEvent` channels (Phase 1 wiring, copied from `bin/hallmark-cli.rs:106-117`).
4. Build popup window (`ui::create_popup_window(&app_handle)`) and companion window (`ui::create_companion_window(&app_handle)`).
5. Construct `AudioDispatcher` + `SchemaCache`.
6. `tokio::spawn(run_watcher(...))`, `tokio::spawn(run_pipeline(...))`, `tokio::spawn(popup_queue::run(...))`, `tokio::spawn(game_detect::run(...))`.

**Wiring pattern reference: `bin/hallmark-cli.rs:99-117`** is the canonical wire-up:
```rust
// ---- Open store + create session ----
let store = Arc::new(SqliteStore::open(&db_path())?);
let session_id = Uuid::new_v4().to_string();
store.with_conn(|conn| queries::create_session(conn, &session_id, None))?;
tracing::info!(session_id = %session_id, "session created");

// ---- Wire channels: watcher ──[raw_*]→ pipeline ──[sink_*]→ stdout printer ----
let (raw_tx, raw_rx) = mpsc::channel::<RawUnlockEvent>(64);
let (sink_tx, mut sink_rx) = mpsc::channel::<RawUnlockEvent>(64);

let watcher_handle = tokio::spawn(run_watcher(adapters, raw_tx));
let pipeline_handle = tokio::spawn(run_pipeline(
    raw_rx,
    store.clone(),
    session_id.clone(),
    sink_tx,
    Duration::from_secs(10),
));
```
**Copy this verbatim into `lib.rs::run()::setup()`.** Then replace the Phase 1 stdout printer with `tokio::spawn(popup_queue::run(app_handle.clone(), sink_rx, schema, audio))`.

---

### `src-tauri/src/store/mod.rs::open()` (modified — apply 002 migration)

**Analog:** `src-tauri/src/store/mod.rs::open` (lines 26-32)

**Phase 1 idempotent migration loader** (from `store/mod.rs:16, 26-32`):
```rust
const INITIAL_MIGRATION_SQL: &str = include_str!("migrations/001_initial.sql");
// ...
pub fn open(db_path: &Path) -> anyhow::Result<Self> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(INITIAL_MIGRATION_SQL)?;
    Ok(Self {
        conn: Mutex::new(conn),
    })
}
```
**Phase 2 minimal extension** — add second migration after 001 (both idempotent, order matters: 001 first):
```rust
const INITIAL_MIGRATION_SQL: &str = include_str!("migrations/001_initial.sql");
const PHASE2_MIGRATION_SQL: &str = include_str!("migrations/002_schema_cache.sql");

pub fn open(db_path: &Path) -> anyhow::Result<Self> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(INITIAL_MIGRATION_SQL)?;
    conn.execute_batch(PHASE2_MIGRATION_SQL)?;
    Ok(Self {
        conn: Mutex::new(conn),
    })
}
```
The same change applies to `open_in_memory()` (line 36-41) so tests get the full schema.

---

### `src-tauri/src/ui.rs` (window builders + HWND patch)

**Analog (closest):** `src-tauri/src/lib.rs::run` (lines 67-78) — the only existing Tauri-API surface

**Why this is the closest match:** Phase 1 has no window-builder code. The `lib.rs::run()` function is the only file that touches `tauri::Builder` / `setup()`. Phase 2 introduces `WebviewWindowBuilder` for the first time; the imports, `#[cfg(target_os = "windows")]` discipline, and tracing conventions are imported from `paths.rs` and `lib.rs`.

**No direct analog code excerpt; structural guidance:**
- File header comment block following Phase 1 convention: `//! Window builders for popup overlay + companion. Phase 2 introduces external borderless windows...`
- Public function shape: `pub fn create_popup_window(app: &AppHandle) -> tauri::Result<()>` — single side-effecting setup call, returns `tauri::Result<()>` (fits Phase 1's mixed `anyhow::Result` + `tauri::Result` conventions; use `tauri::Result` here because Tauri-domain).
- `#[cfg(target_os = "windows")]` for the HWND patch block (from `paths.rs::read_steam_install` analog above) — non-Windows is a no-op so the rest of the file stays cross-compilable.
- Use the full `WebviewWindowBuilder` chain from RESEARCH.md Pattern 1 (lines 332-382) verbatim — that pattern is verified against Tauri 2.11 docs.

**Phase 1 import-style example to mirror** (from `paths.rs:21-23`):
```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};
```
For Phase 2 ui.rs:
```rust
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW,
        GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
    },
};
```

---

### Frontend files (`src/**/*.tsx`, `package.json`, `vite.config.ts`)

**No analog in Phase 1** — this is the project's first React/TS surface.

**Type-mirroring pattern** for `src/types.ts`:
- `src-tauri/src/sources/mod.rs::RawUnlockEvent` (lines 33-45) is the canonical Rust event shape; popup payloads sent via `app.emit_to(...)` will serde-serialize as JSON. The TS type must mirror the Rust struct's serialized field names exactly.
- The `PopupPayload` struct in RESEARCH.md Pattern 4 (lines 547-557) is the authoritative serialized contract.

**Use RESEARCH.md as the primary reference** for all frontend code:
- Pattern 5 (lines 622-695) for `PopupRoot` + spring choreography.
- UI-SPEC.md "Component Inventory" table for component breakdown.
- UI-SPEC.md "Color" + "Typography" + "Spacing Scale" sections for styles.
- UI-SPEC.md "Copywriting Contract" for all literal strings.

---

## Shared Patterns

### Tracing (cross-cutting — apply to all new Rust files)

**Source:** Used uniformly across Phase 1 — see `lib.rs::init_tracing` (lines 22-33), `watcher/mod.rs:115-119`, `goldberg.rs:227-233`, `paths.rs:106-128`.

**Apply to:** Every new Rust module (`ui.rs`, `popup_queue.rs`, `audio.rs`, `game_detect/*.rs`, `schema/*.rs`, `monitor.rs`).

**Pattern — structured field-keyed tracing:**
```rust
tracing::info!(
    app_id = evt.app_id,
    ach = %evt.ach_api_name,
    source = %evt.source,
    "UNLOCK"
);
```
- Use `=` for owned/Debug values; `= %...` for `Display` impls; `= ?...` for `Debug` (rare).
- The terminal string is the event name (uppercase verb or short phrase). Field keys precede.
- Levels: `trace!` (per-byte detail), `debug!` (per-event detail), `info!` (lifecycle: start, stop, key transitions), `warn!` (recoverable failure with `error = %e`), `error!` (system invariant violation).
- Never log without context fields when emitting from a background task; the field-keyed style is what makes WatcherCore + Goldberg debuggable.

### Error handling (cross-cutting — apply to all new Rust files)

**Source:** Pattern shared across `goldberg.rs:175-187`, `paths.rs:189-198`, `watcher/mod.rs:131-136`.

**Apply to:** All Rust modules that perform I/O or fallible parsing.

**Pattern — match-on-Result with continue + warn rather than `?`-propagation in event loops:**
```rust
let json = match read_with_retry(path).await {
    Ok(s) => s,
    Err(e) => {
        tracing::warn!(path = %path.display(), error = %e, "read failed; skip");
        continue;  // or `return Ok(())` for non-loop fns
    }
};
```
**Rule:** in long-running tasks (`run_watcher`, `run_pipeline`, `popup_queue::run`, `game_detect::run`), a single error must NOT terminate the task. Convert to `tracing::warn!` + skip-this-event. `?`-propagation is reserved for `setup()`-time failures where the app cannot start.

**`anyhow::Result` everywhere except Tauri-domain:** Phase 1 uses `anyhow::Result<T>` for every fallible function. The exception is `tauri::Result<()>` for window-builder functions in `ui.rs`. Convert at the boundary: `let win = WebviewWindowBuilder::...build().map_err(anyhow::Error::from)?` if calling from anyhow context.

### `Arc<Mutex<...>>` shared mutable state (cross-cutting — apply to popup_queue, audio, schema)

**Source:** `store/mod.rs:20-21` (`Mutex<Connection>`), `goldberg.rs:67-69` (`Arc<RwLock<HashMap<...>>>`).

**Apply to:** `popup_queue` (queue depth tracking, last-payload state), `schema::cache` (in-memory cache layer if added on top of SQLite), `game_detect` (current-game state shared with HWND lookup task).

**Pattern — std `Mutex` for sync-only access (SQLite-style), tokio `Mutex` for await-spanning access:**
```rust
// std::sync::Mutex — see store/mod.rs
pub struct SqliteStore {
    pub(super) conn: Mutex<Connection>,
}

// tokio::sync::Mutex — see watcher/mod.rs:339, 357
let dedup = Arc::new(TokioMutex::new(CrossSourceDedup::new(dedup_ttl)));
```
**Decision rule:** if the lock guard outlives an `.await`, use `tokio::sync::Mutex`. Otherwise prefer `std::sync::Mutex` (faster, no async overhead). Phase 1 demonstrates both.

**Poison recovery** (from `store/mod.rs:70`):
```rust
let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
```
Apply to all std `Mutex` consumers in long-running daemons. The release profile sets `panic = "abort"` (workspace `Cargo.toml` line 17), which sidesteps poisoning in production but the recovery is still belt-and-suspenders for debug.

### Test fixtures (cross-cutting — apply to all new Rust modules with tests)

**Source:** `goldberg.rs:367-387` (`fresh_tmp` helper), `queries.rs:81-83` (`fresh_store`), `paths.rs:662-666` (`fresh_tmp(name)`), `watcher/mod.rs:234-238` (`fresh_tmp`).

**Apply to:** Any new module with `#[cfg(test)] mod tests`.

**Pattern — uuid-tagged temp dirs for parallel-safe tests:**
```rust
fn fresh_tmp() -> PathBuf {
    let p = std::env::temp_dir().join(format!("hallmark-watcher-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&p).unwrap();
    p
}
```
**Variant for named scope** (from `paths.rs:662`):
```rust
fn fresh_tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("hallmark-{}-{}", name, uuid::Uuid::new_v4()));
    fs::create_dir_all(&p).unwrap();
    p
}
```
**In-memory SQLite for query tests** (from `queries.rs:81-83`):
```rust
fn fresh_store() -> SqliteStore {
    SqliteStore::open_in_memory().unwrap()
}
```

**Cleanup convention:** every test ends with `let _ = fs::remove_dir_all(&root);` (silently ignore failures — the OS will clean tmp eventually).

### Tauri capabilities (cross-cutting — applies to popup.json + companion.json)

**No analog in Phase 1** — Phase 1 has no capabilities files (Tauri shell starts with empty `windows` array, no permissions needed).

**Use RESEARCH.md as canonical:**
- popup capability — `core:event:allow-listen` only (popup is non-interactive; just listens for `popup-show`/`popup-hide`).
- companion capability — `core:event:allow-listen` + `core:window:allow-show` + `core:window:allow-hide` + `core:window:allow-set-size` + `core:window:allow-set-position` + `core:window:allow-start-dragging` (custom drag region per UI-SPEC.md `CompanionHeader`) + `core:webview:allow-internal-toggle-devtools` (dev only) + asset protocol for icon images.

### CSP locked-down outbound (cross-cutting — applies to tauri.conf.json)

**Source:** RESEARCH.md "Tauri config" section — net-new in Phase 2.

**Pattern:**
```json
"csp": "default-src 'self'; img-src 'self' data: https://media.steampowered.com https://cdn.akamai.steamstatic.com; connect-src 'self' https://api.steampowered.com"
```
**Rule:** all outbound HTTP from the WebView is blocked except the locked-down list. Rust-side `reqwest` is unaffected by CSP. Phase 1 uses `csp: null` (placeholder); Phase 2 replaces with the lockdown above.

### IN-03 u64 → i64 overflow guard (cross-cutting — apply to all SQLite writes that take an app_id)

**Source:** `store/mod.rs:64-65`, `queries.rs:23-25, 56`.

**Pattern:**
```rust
let app_id_i64 = i64::try_from(app_id)?;
```
**Rule:** every typed SQLite helper that accepts a u64 app_id must use `try_from` rather than `as i64`. Steam app IDs fit in 32 bits today; the guard is a forward-compat trap for non-Steam u64 sources.

---

## No Analog Found

Files with no close match in the codebase. Planner should use RESEARCH.md patterns as the primary reference; there is nothing to copy from Phase 1.

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `package.json` | manifest | config | Phase 1 has no JS/TS surface (`dist/index.html` is a placeholder; no Vite, React, or Node) |
| `vite.config.ts` | config | config | Same — no existing Vite config |
| `tsconfig.json` | config | config | Same — no existing TS config |
| `popup.html`, `index.html` (companion) | entry | static | The placeholder `dist/index.html` (Phase 1 IN-06) is to be replaced wholesale; not a pattern to copy |
| `src/main-popup.tsx`, `src/main-companion.tsx` | entry | event-driven | No React entry exists yet |
| `src/components/*.tsx` | components | request-response | No components exist yet |
| `src/hooks/*.ts` | hooks | event-driven | No hooks exist yet |
| `src/styles/*.css` | stylesheets | static | No styles exist yet |
| `src-tauri/capabilities/popup.json` | config | config | Phase 1 has no capabilities directory |
| `src-tauri/capabilities/companion.json` | config | config | Same |
| `src-tauri/src/schema/steam_api.rs` | service (HTTP) | request-response | Phase 1 makes no HTTP requests (no `reqwest` dep yet) |
| `src-tauri/src/audio.rs` | service (audio) | request-response | Phase 1 plays no audio (no `rodio` dep yet) |
| `src-tauri/src/game_detect/process_scan.rs` | service (sysinfo) | batch | Phase 1 enumerates no processes (no `sysinfo` dep yet) |
| `src-tauri/src/monitor.rs` (windows-rs Win32 calls beyond winreg) | utility | request-response | Phase 1 uses winreg only; HWND/MonitorFromWindow/EnumWindows are net-new |
| `assets/sfx/*.wav`, `assets/icons/placeholder.png` | static asset | static | Phase 1 has only `src-tauri/icons/icon.ico` (placeholder per IN-06) |

For the Rust files in this list (`steam_api.rs`, `audio.rs`, `process_scan.rs`, `monitor.rs`), apply the **shared patterns** above (tracing, error handling, Arc<Mutex>, test fixtures, IN-03 guard) plus the RESEARCH.md code samples for the domain-specific work.

---

## Metadata

**Analog search scope:**
- `src-tauri/src/**/*.rs` (10 files)
- `src-tauri/src/store/migrations/*.sql` (1 file)
- `src-tauri/Cargo.toml`, `Cargo.toml`, `src-tauri/tauri.conf.json` (config)
- `dist/index.html` (placeholder — confirmed not a pattern to copy)

**Files scanned:** 14 source files in `src-tauri/`, 4 config files, 1 SQL migration. Frontend: zero existing files (confirmed — `dist/index.html` is the only HTML and it's a Phase 1 IN-06 placeholder).

**Pattern extraction date:** 2026-05-08

**Key architectural insight for the planner:** Phase 1 established a small but coherent set of conventions — `Arc<dyn SourceAdapter>` for pluggable mechanisms, `mpsc::Receiver`-driven drain tasks, `with_conn(closure)` for SQLite extension, idempotent `IF NOT EXISTS` migrations, `#[cfg(target_os = "windows")]` arms with non-Windows stubs, structured field-keyed tracing, `anyhow::Result` everywhere except Tauri-domain, uuid-tagged tmp fixtures. Phase 2's Rust modules slot directly into these conventions without inventing new ones. The frontend is genuinely net-new and must rely on RESEARCH.md + UI-SPEC.md as primary references.
