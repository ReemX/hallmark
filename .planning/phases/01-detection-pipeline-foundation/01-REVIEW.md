---
phase: 01-detection-pipeline-foundation
reviewed: 2026-05-08T00:00:00Z
depth: standard
files_reviewed: 17
files_reviewed_list:
  - src-tauri/src/main.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/error.rs
  - src-tauri/src/paths.rs
  - src-tauri/src/sources/mod.rs
  - src-tauri/src/sources/goldberg.rs
  - src-tauri/src/store/mod.rs
  - src-tauri/src/store/queries.rs
  - src-tauri/src/store/migrations/001_initial.sql
  - src-tauri/src/watcher/mod.rs
  - src-tauri/src/watcher/dedup.rs
  - src-tauri/src/bin/hallmark-cli.rs
  - src-tauri/tests/integration_phase1.rs
  - src-tauri/build.rs
  - src-tauri/Cargo.toml
  - src-tauri/tauri.conf.json
  - Cargo.toml
findings:
  blocker: 4
  warning: 11
  info: 6
  total: 21
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-05-08
**Depth:** standard
**Files Reviewed:** 17
**Status:** issues_found

## Summary

Phase 1 implements the Goldberg detection pipeline (path discovery → adapter → watcher → dedup → SQLite). Overall structure is well-organised and tested, but the review surfaced multiple correctness defects that bite on Windows hardware:

1. **Case-sensitivity bug** in `appmanifest_lookup` silently drops `local_save.txt` redirects when the on-disk install directory differs in case from the appmanifest's `installdir` value (Windows is case-insensitive — this WILL happen in practice).
2. **Hash-then-parse ordering** in `Goldberg::on_file_changed` updates `last_hash` BEFORE parsing succeeds, so a transient/partial-write read short-circuits the next legitimate read.
3. **Path-traversal exposure** through unsanitised `local_save.txt` content (attacker-controlled redirect can point anywhere on disk; we then read JSON from there).
4. **`dispatch()` re-invokes `watch_paths()` on every event**, which re-runs `path.exists()` syscalls. On Goldberg this also means events are silently dropped if the watched root was removed mid-session.

Beyond these, several quality issues will accumulate cost: dead error types (`error.rs` is unused), `Mutex`/`unwrap()` panic-on-poison patterns in the store, an unused `_adapters` parameter in `run_pipeline`, and a `last_err.unwrap()` in `read_with_retry` whose safety relies on a non-obvious invariant.

No SQL injection (queries are parameterised). No hardcoded credentials. Dependency versions in `Cargo.toml` deviate from the documented stack and at least one (`sha2 = "0.11"`) may not exist as a published version — verify before proceeding.

---

## Blocker Issues

### BL-01: Case-insensitive `installdir` lookup fails on Windows

**File:** `src-tauri/src/paths.rs:367-369` (insert) and `:469-479` (lookup)
**Issue:**
`appmanifest_lookup` builds a `HashMap<String, u64>` keyed on the EXACT-CASE `installdir` string from the appmanifest. `extract_installdir_from_dll_path` then returns the EXACT-CASE on-disk directory segment after `common`. The two strings can differ in case on Windows because:
- Windows filesystems are case-INSENSITIVE; `D:\SteamLibrary\steamapps\common\Foogame` and `Foogame` (or any case mix) refer to the same directory.
- Steam may write `installdir` with one case and the user's actual on-disk directory may be different (e.g., user copied a backup, file system normalisation differs across SMB shares, etc.).

When the cases do not match, `manifest_map.get(&installdir)` returns `None`, the redirect is logged with `"no appmanifest_*.acf matches installdir"` and DROPPED. This breaks REQ DETECT-08's stated guarantee that every `local_save.txt` redirect resolves an appid.

The existing tests pass because the test harness deliberately uses identical strings on both sides (e.g. `"FooGame"` in both `write_appmanifest` and the directory `game_bin = common.join("FooGame")...`).

**Fix:**
Either normalise the key on insert and lookup with `to_ascii_lowercase()`, or use a case-insensitive map. Minimal change:
```rust
// In appmanifest_lookup — insert lowercased key:
if let (Some(id), Some(dir)) = (appid, installdir) {
    map.insert(dir.to_ascii_lowercase(), id);
}

// In scan_local_save_redirects — lookup lowercased:
let app_id = match manifest_map.get(&installdir.to_ascii_lowercase()).copied() {
    Some(id) => id,
    None => { /* warn + skip */ }
};
```
Add a regression test that writes `installdir = "FooGame"` to the appmanifest but creates the on-disk directory as `foogame`, then asserts the redirect resolves correctly.

### BL-02: `last_hash` updated before parse — invalid intermediate write poisons next event

**File:** `src-tauri/src/sources/goldberg.rs:250-266`
**Issue:**
The order in `on_file_changed` is:
1. Read file (line 241).
2. Compute SHA-256, INSERT into `last_hash` (lines 250-258).
3. Parse JSON (line 260).
4. If parse fails, return Ok(()) (line 263-265).

If step 1 catches a partial/in-progress write (Goldberg writes the file open-write-close, see PITFALLS.md #3 referenced in `read_with_retry`), the JSON may be malformed. We compute its hash, persist it as the "last seen" hash, and return early. A second debounced event for the SAME file with the SAME malformed bytes (extremely unlikely but possible) would short-circuit on hash equality and never see the eventual valid write. More practically: if the next event arrives carrying the (now-valid) full file content with a hash that happens to differ from the malformed intermediate (which it almost always will), we correctly process it. So the worst case is rare but real.

The deeper problem is the **invariant violation**: `last_hash` is meant to mean "we have already processed this content"; after a parse failure we have NOT processed it, but the cache says we have.

Closely related: even on the success path, the `last_hash` write lock and `baseline` write lock are acquired separately (lines 252 and 269) — between them another `on_file_changed` call could race. With a single-threaded executor this never interleaves, but the code does not document or enforce that assumption.

**Fix:**
Move the `last_hash.insert()` to AFTER successful parse + diff:
```rust
// Compute hash but defer the insert.
let hash: [u8; 32] = Sha256::digest(json.as_bytes()).into();
{
    let hashes = self.last_hash.read().await;
    if hashes.get(&path) == Some(&hash) {
        tracing::trace!(path = %path.display(), "content unchanged; skip");
        return Ok(());
    }
}

let state = match Self::parse_state(&json) {
    Ok(s) => s,
    Err(e) => {
        tracing::warn!(path = %path.display(), error = %e, "state file parse failed");
        return Ok(()); // do NOT update last_hash on parse failure
    }
};

// Diff + emit (existing logic) ...

// Only NOW commit the hash so future identical reads short-circuit.
self.last_hash.write().await.insert(path.clone(), hash);
```

### BL-03: `dispatch()` calls `adapter.watch_paths()` per event — re-stats the filesystem and silently drops events when a root is removed

**File:** `src-tauri/src/watcher/mod.rs:120-129` (dispatch) and `src-tauri/src/sources/goldberg.rs:116-125` (watch_paths)
**Issue:**
`dispatch()` iterates each adapter and calls `adapter.watch_paths()` to find the prefix-match. `GoldbergAdapter::watch_paths()` invokes `path.exists()` for every root and every redirect-map entry — that is, a syscall per watched directory PER FILESYSTEM EVENT.

Two concrete problems:
1. **Correctness:** If a watched root or redirect parent is renamed/deleted between watcher startup and an event, `watch_paths()` filters it out via `.filter(|p| p.exists())`. The previously-registered watch handle in the debouncer still fires events for paths under it, but `dispatch()` no longer prefix-matches and the event is silently dropped at `tracing::trace!("no adapter claims this path; ignoring")`. Critical achievements would be lost.
2. **Quality (out of scope but adjacent):** Allocating a `Vec<PathBuf>` per event with N stat() calls inside the hot path for every FS event is wasteful and the kind of thing CI flame-graph regressions catch later.

**Fix:**
Cache the watch path set at watcher startup. `WatcherCore` already iterates `adapter.watch_paths()` once during setup (lines 66-85). Capture that into a `Vec<(adapter_idx, PathBuf)>` and reuse it in `dispatch()`:
```rust
let mut path_owner: Vec<(usize, PathBuf)> = Vec::new();
for (idx, adapter) in adapters.iter().enumerate() {
    for path in adapter.watch_paths() {
        if !path.exists() { continue; }
        if debouncer.watch(&path, RecursiveMode::Recursive).is_ok() {
            path_owner.push((idx, path));
        }
    }
}
// In dispatch: scan path_owner instead of re-calling watch_paths().
```
Additionally, do NOT filter out matches based on `path.exists()` at dispatch time — once a watch was registered, treat it as valid for the watcher's lifetime even if the directory disappears.

### BL-04: Path traversal via attacker-controlled `local_save.txt`

**File:** `src-tauri/src/paths.rs:429-456`
**Issue:**
`scan_local_save_redirects` reads `local_save.txt` from a game's install directory and trusts its contents as a filesystem path. If `trimmed` is absolute it is used as-is; if relative it is `dir.join(trimmed)` with no traversal protection. An attacker-controlled or malicious `local_save.txt` containing `..\..\..\Users\<user>\AppData\Roaming\Microsoft\Credentials` (or any directory with a JSON file) would cause Hallmark to register a recursive `notify` watch and read JSON from that location.

The blast radius is currently bounded — we only:
- `walkdir` looking for `achievements.json`
- `serde_json::from_str::<HashMap<String, GoldbergEntry>>` on file contents

We do not write to or execute code from the redirect target. But:
- We DO log full resolved paths at `tracing::info!` (line 481-487), including paths like `C:\Users\<user>\AppData\Roaming\Microsoft\...` if a malicious `local_save.txt` redirects there. Information disclosure into log files.
- We DO keep a recursive `ReadDirectoryChangesW` watch on the target indefinitely.
- Attacker-controlled directory listings can be reflected via tracing logs.

A user installing a tampered Goldberg-emulated game does not realistically expect this to expose unrelated directories.

**Fix:**
Validate that the resolved path is within an expected scope. Two options:
1. **Strict containment** — require `resolved` to be a descendant of either the DLL's parent directory or an allow-listed Goldberg root. Reject otherwise.
2. **Canonicalise + sanity check** — after `Path::canonicalize`, ensure no ancestor matches a deny list (Windows / Program Files / system32 / Users\*\AppData\Local\Microsoft / etc.).

Minimum viable fix:
```rust
let resolved = if Path::new(trimmed).is_absolute() {
    PathBuf::from(trimmed)
} else {
    dir.join(trimmed)
};
let canon = match resolved.canonicalize() {
    Ok(c) => c,
    Err(e) => {
        tracing::warn!(unresolved = %resolved.display(), error = %e,
            "local_save.txt target failed to canonicalize; skipping");
        continue;
    }
};
// Reject if canon escapes the game install directory AND is not under a known Goldberg root.
let dll_canon = dir.canonicalize().ok();
if let Some(dc) = &dll_canon {
    if !canon.starts_with(dc) && !canon.starts_with(/* %APPDATA% */) {
        tracing::warn!(target = %canon.display(),
            "local_save.txt redirect points outside game dir and known Goldberg roots; refusing");
        continue;
    }
}
```

---

## Warnings

### WR-01: `error.rs` defines three error enums that are never used

**File:** `src-tauri/src/error.rs:1-36`
**Issue:**
`PathDiscoveryError`, `AdapterError`, and `StoreError` are declared `pub enum` with `thiserror::Error`, exported via `pub mod error` in `lib.rs:8`, but a `Grep` across the entire `src/` tree shows zero use sites. Production code returns `anyhow::Result` everywhere. The module's own doc comment ("library boundaries return these typed errors") is at odds with the actual code.

**Fix:**
Either:
- Delete `error.rs` entirely (and remove `pub mod error` from `lib.rs`).
- OR migrate at least one library boundary (e.g. `paths::discover` → `Result<DiscoveredPaths, PathDiscoveryError>`, `SourceAdapter::seed_baseline` → `Result<(), AdapterError>`) so the types pull their weight.

Leaving them as dead code invites drift between intent and reality.

### WR-02: `run_pipeline` accepts an `_adapters` parameter it never uses

**File:** `src-tauri/src/watcher/mod.rs:303-310`
**Issue:**
```rust
pub async fn run_pipeline(
    _adapters: Vec<Arc<dyn SourceAdapter>>, // not directly used here, kept for API symmetry
    ...
) -> anyhow::Result<()>
```
The parameter is intentionally unused with a leading underscore. "API symmetry" is not a justification — callers (the CLI binary, `integration_phase1.rs::spawn_pipeline`) must construct and clone an unused `Vec<Arc<dyn SourceAdapter>>`. This is friction with no benefit, and increases the chance someone passes a wrong/empty Vec because it never matters.

**Fix:**
Remove the parameter:
```rust
pub async fn run_pipeline(
    mut raw_rx: mpsc::Receiver<RawUnlockEvent>,
    store: Arc<SqliteStore>,
    session_id: String,
    sink: mpsc::Sender<RawUnlockEvent>,
    dedup_ttl: Duration,
) -> anyhow::Result<()> { ... }
```
Update callers in `bin/hallmark-cli.rs:110-117` and `tests/integration_phase1.rs:71-80`.

### WR-03: `read_with_retry` panics if loop body never executes (not currently triggerable, but fragile)

**File:** `src-tauri/src/sources/goldberg.rs:292-308`
**Issue:**
```rust
fn read_with_retry(path: &Path) -> anyhow::Result<String> {
    let mut last_err: Option<std::io::Error> = None;
    for _ in 0..3 {
        match std::fs::read_to_string(path) {
            Ok(s) => return Ok(s),
            Err(e) if /* retryable */ => { last_err = Some(e); std::thread::sleep(...); }
            Err(e) => return Err(e.into()),
        }
    }
    Err(last_err.unwrap().into())
}
```
The `unwrap()` on the last line is safe only because the `0..3` loop guarantees at least one iteration AND the only paths that exit the loop without returning are those that set `last_err = Some(e)`. Change the `0..3` literal to `0..0` (or any expression that could be 0) and the unwrap panics. A future refactor that derives the retry count from a config parameter could silently introduce a panic.

**Fix:**
Replace the unwrap with a non-panicking match:
```rust
match last_err {
    Some(e) => Err(e.into()),
    None => Err(anyhow::anyhow!("read_with_retry: 0 attempts configured; refusing")),
}
```
Or restructure so the type system enforces the invariant (e.g., split into `try_once` then `try_with_retry` that initialises last_err from the first call).

### WR-04: `read_with_retry` uses `std::thread::sleep` inside an async context

**File:** `src-tauri/src/sources/goldberg.rs:302`
**Issue:**
`read_with_retry` is invoked from `async fn on_file_changed` via direct call (lines 161, 191, 241). When the retryable error path triggers, `std::thread::sleep(Duration::from_millis(50))` blocks the entire tokio worker thread for up to 150ms (3 × 50ms). On a multi-threaded runtime this stalls one worker; in tests using `#[tokio::test]` (single-threaded by default), it stalls the whole test runtime.

**Fix:**
Either make `read_with_retry` async and use `tokio::time::sleep`, or call it via `tokio::task::spawn_blocking` from the adapter:
```rust
async fn read_with_retry_async(path: PathBuf) -> anyhow::Result<String> {
    tokio::task::spawn_blocking(move || {
        let mut last_err = None;
        for _ in 0..3 {
            match std::fs::read_to_string(&path) {
                Ok(s) => return Ok(s),
                Err(e) if /* retryable */ => {
                    last_err = Some(e);
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => return Err(e.into()),
            }
        }
        Err(last_err.expect("loop ran at least once").into())
    }).await?
}
```

### WR-05: `SqliteStore` mutex panics on poisoning, and `with_conn` holds the mutex across user-supplied closures

**File:** `src-tauri/src/store/mod.rs:60, 72, 86`
**Issue:**
Every call site does `self.conn.lock().unwrap()`. If a previous closure panicked while holding the lock (e.g., a query helper in `queries.rs` panics because of a malformed argument), the mutex becomes poisoned and every subsequent call panics on `.unwrap()`. For a long-running daemon process this means a single bad query takes down the whole pipeline.

Additionally, `with_conn(|conn| f(conn))` holds the lock for the entire duration of `f`. The doc says "keep the closure short" but nothing enforces it; a multi-statement transaction or anything that awaits cannot run in this closure (it's sync-only) but a long-running query will starve other paths.

**Fix:**
1. Replace `.lock().unwrap()` with `.lock().unwrap_or_else(|p| p.into_inner())` (recover from poisoning).
2. Document the synchronous-only contract in the closure type and consider returning the lock guard via a typed accessor for transactional use.
3. Long-term: switch to `tokio::sync::Mutex` for the connection (sqlite is thread-safe with `bundled` + serialized mode) so async callers can acquire across await points if needed.

### WR-06: `sha2 = "0.11"` may not be a published crate version

**File:** `src-tauri/Cargo.toml:26`
**Issue:**
`sha2 = "0.11"` is requested. As of late 2025 the latest stable release is `0.10.x`; `0.11` is not on crates.io for the `sha2` crate. The CLAUDE.md technology stack does not list a `sha2` version, so this looks like an unverified guess. If `0.11` is unpublished, `cargo build` fails immediately. If `0.11` is published as a pre-release, it has not been audited against the rest of the stack.

Similarly verify:
- `notify-debouncer-full = "0.7"` — CLAUDE.md recommends `0.5` (currently stable).
- `rusqlite = "0.39"` — verify this exists; latest as of recent times has been `0.31.x`–`0.32.x`.
- `tokio = "1.52"` — newer than typical; verify availability.
- `dirs = "6.0"` — historical version is `5.x`; check whether `6.0` exists.

**Fix:**
Run `cargo search <crate>` for each dependency and pin to a published, audited version. Update `Cargo.lock` and verify `cargo build` succeeds in CI before merging.

### WR-07: `tracing_subscriber::fmt().try_init()` errors silently ignored

**File:** `src-tauri/src/lib.rs:21-25`
**Issue:**
```rust
let _ = tracing_subscriber::fmt()
    .with_env_filter(filter)
    .with_target(true)
    .with_level(true)
    .try_init();
```
`try_init` returns `Err` if a subscriber is already installed. In tests this is desired (multiple tests call `init_tracing`). In production, if `try_init` fails for ANY reason (already-installed because we initialized twice in the same process, broken global state) we silently lose all logging — yet the `tracing::info!("Hallmark starting...")` immediately after will appear to "succeed" (it just routes to a no-op subscriber).

**Fix:**
Differentiate test vs. production:
```rust
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("hallmark_lib=info,warn"));
    if let Err(e) = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_level(true)
        .try_init()
    {
        eprintln!("WARNING: tracing init failed: {e}");
    }
}
```
For tests, expose an `init_tracing_for_tests()` that swallows the error explicitly.

### WR-08: `Goldberg::watch_paths()` silently filters out non-existent paths — events from registered-then-deleted paths are lost

**File:** `src-tauri/src/sources/goldberg.rs:116-125`
**Issue:**
The function filters `self.roots` and `self.redirect_map` keys by `.exists()`. This is invoked on every `dispatch()` call (per WR-03). If a Goldberg save root existed at adapter creation but was deleted (user uninstalled the emulator, OS cleaned a temp dir, etc.), pending FS events for that directory will no longer prefix-match in `dispatch()` and will be silently dropped without a tracing entry above `trace`.

**Fix:**
Cache the path set at adapter construction (resolves at startup, never changes during runtime). Combined with WR-03 fix:
```rust
pub struct GoldbergAdapter {
    roots: Vec<PathBuf>,
    redirect_map: HashMap<PathBuf, u64>,
    cached_watch_paths: Vec<PathBuf>, // set in `new()`
    ...
}

impl GoldbergAdapter {
    pub fn new(roots: Vec<PathBuf>, redirect_map: HashMap<PathBuf, u64>) -> Self {
        let mut cached: Vec<PathBuf> = roots.iter().filter(|p| p.exists()).cloned().collect();
        for k in redirect_map.keys() {
            if k.exists() && !cached.contains(k) { cached.push(k.clone()); }
        }
        Self { roots, redirect_map, cached_watch_paths: cached, ... }
    }
}

fn watch_paths(&self) -> Vec<PathBuf> {
    self.cached_watch_paths.clone()
}
```

### WR-09: First-prefix-match wins in `dispatch()` is undefined behaviour when adapters overlap

**File:** `src-tauri/src/watcher/mod.rs:121-128`
**Issue:**
The comment says "adapters MUST not have overlapping roots" but nothing checks. Phase 3 will add SteamLegit, CreamApi, SmartSteamEmu — each with their own watch paths. If a user has Goldberg redirecting INTO `%APPDATA%\Steam\CODEX\` (or any path that overlaps another adapter's roots), `dispatch()` silently routes to whichever adapter happened to be earlier in the `Vec`. Adapters cannot detect this misconfiguration.

Order of insertion into the adapter Vec is currently determined by the CLI binary, which inserts only Goldberg today; future work (Phase 3) will multiplex.

**Fix:**
At watcher startup, validate that no two adapters' watch paths share a prefix relationship (one is an ancestor of another). If overlap is detected, log a `tracing::error!` and either refuse to start or let both adapters receive the event:
```rust
// Detection at startup:
for (i, ai) in adapters.iter().enumerate() {
    for pa in ai.watch_paths() {
        for (j, aj) in adapters.iter().enumerate() {
            if i == j { continue; }
            for pb in aj.watch_paths() {
                if pa.starts_with(&pb) || pb.starts_with(&pa) {
                    tracing::error!(adapter_a = ai.name(), adapter_b = aj.name(),
                        path_a = %pa.display(), path_b = %pb.display(),
                        "adapter watch paths overlap; events may be misrouted");
                }
            }
        }
    }
}
```
For routing, dispatch to ALL prefix-matching adapters (let dedup handle the duplicate downstream — that is its job).

### WR-10: `MockAdapter` in `integration_phase1.rs` has a TOCTOU window on baseline updates

**File:** `src-tauri/tests/integration_phase1.rs:324-353`
**Issue:**
```rust
let was = self.baseline.read().await.unwrap_or(false);
if !was && earned_now {
    let _ = tx.send(RawUnlockEvent { ... }).await;
}
*self.baseline.write().await = Some(earned_now);
```
Read lock is dropped before the write, with two await points (`tx.send(...)`, `.write().await`) between read and write. Two concurrent invocations on the same `MockAdapter` can both observe `was = false`, both send the event, and both proceed to write. The dedup stage downstream is supposed to catch this — but if the test ever runs without the pipeline (or misconfigures TTL), it would falsely emit two events.

This is a test-only concern but the same pattern exists in production-style code. Worth fixing as a hygiene matter so the test reflects the contract `GoldbergAdapter` upholds.

**Fix:**
Hold the write lock across the read+emit+update sequence:
```rust
let mut baseline = self.baseline.write().await;
let was = baseline.unwrap_or(false);
if !was && earned_now {
    let _ = tx.send(...).await;
}
*baseline = Some(earned_now);
```

### WR-11: SQLite UNIQUE INDEX on `(app_id, ach_api_name, session_id)` does not protect against NULL session_id

**File:** `src-tauri/src/store/migrations/001_initial.sql:22-23` and `src-tauri/src/store/mod.rs:144-156`
**Issue:**
The migration says (correctly) that SQLite treats NULL as distinct from NULL in UNIQUE INDEX. The test `record_unlock_null_session_treated_as_distinct` confirms two NULL inserts produce two rows. The doc comment says "Production code (Plan 05) always passes Some(_)", but the API allows `None` and there is no runtime guard. A bug elsewhere that drops the session_id would silently disable the dedup constraint and double-record every unlock.

**Fix:**
Either:
1. Make `session_id` NOT NULL in the schema:
   ```sql
   session_id  TEXT NOT NULL,
   ```
   And make the API require `&str` instead of `Option<&str>`.
2. OR add a CHECK constraint:
   ```sql
   CHECK (session_id IS NOT NULL)
   ```
3. OR add a partial unique index for the NULL case:
   ```sql
   CREATE UNIQUE INDEX idx_unlock_dedup_null_session
       ON unlock_history(app_id, ach_api_name) WHERE session_id IS NULL;
   ```
Option 1 is cleanest given the doc says session is always present.

---

## Info

### IN-01: `ERROR_LOCK_VIOLATION` (33) not retried alongside `ERROR_SHARING_VIOLATION` (32)

**File:** `src-tauri/src/sources/goldberg.rs:298-300`
**Issue:**
The retry condition checks for `PermissionDenied` and `raw_os_error() == Some(32)`. Windows uses `ERROR_LOCK_VIOLATION = 33` for distinct locking scenarios that may also occur during Goldberg's open-write-close cycle. Worth handling the same way.

**Fix:**
```rust
Err(e)
    if e.kind() == std::io::ErrorKind::PermissionDenied
        || matches!(e.raw_os_error(), Some(32) | Some(33))
=> { ... }
```

### IN-02: `walkdir::WalkDir::new(&common).max_depth(8)` may miss DLLs deeper than 8 levels

**File:** `src-tauri/src/paths.rs:410`
**Issue:**
The comment claims "depth 2-4" is typical but some games (especially modded distributions) nest DLLs deeper (engine/redists/binaries/win64/x86_64/...). Hard-coded depth 8 is a magic number with no constant.

**Fix:**
Either bump to 12 (still bounded) and document the trade-off, or extract `const STEAMAPI_MAX_SEARCH_DEPTH: usize = 8;` so the value is reviewable.

### IN-03: `app_id as i64` cast loses upper bit

**File:** `src-tauri/src/store/mod.rs:65` and `src-tauri/src/store/queries.rs:24, 53`
**Issue:**
Steam app IDs are 32-bit unsigned, so the cast is always lossless in practice. But the type is `u64` and the cast is silent. Future code or a non-Steam source emitting larger IDs would wrap to negative.

**Fix:**
Use `i64::try_from(app_id)?` or document the constraint with a `debug_assert!(app_id <= i64::MAX as u64)`.

### IN-04: `with_conn` doc claims "mutex held for duration of closure" but does not enforce closure cannot panic

**File:** `src-tauri/src/store/mod.rs:82-88`
**Issue:**
Combined with WR-05 — closure panic poisons the mutex permanently. Worth a doc note pointing readers at `panic = "abort"` (the workspace `Cargo.toml` sets this for release builds, mitigating the risk) and recommending `catch_unwind` for closures that could conceivably panic.

**Fix:**
Add doc comment:
```rust
/// NOTE: If the closure panics, the underlying Mutex becomes poisoned and all
/// subsequent calls return the poison error. Workspace `Cargo.toml` sets
/// `panic = "abort"` in release builds, which sidesteps this; but in debug
/// builds and tests, callers should ensure their closures cannot panic.
```

### IN-05: `notify-debouncer-full = "0.7"` deviates from the version recommended in CLAUDE.md (0.5)

**File:** `src-tauri/Cargo.toml:20`
**Issue:**
CLAUDE.md technology stack (lines about notify) recommends `notify-debouncer-full = 0.5` (stable) or `9.0.0-rc.4` (next). The project uses `0.7` without justification. May be fine — `0.7` could exist and improve on `0.5` — but if so, update CLAUDE.md to match. Otherwise downgrade.

**Fix:**
Either pin to `0.5` per CLAUDE.md, or update CLAUDE.md to reflect the chosen version with rationale (and verify `0.7` exists on crates.io).

### IN-06: `tauri.conf.json` has empty `windows: []` and no CSP — fine for Phase 1, document for later

**File:** `src-tauri/tauri.conf.json:13-15`
**Issue:**
`"windows": []` and `"security": { "csp": null }` are intentional for Phase 1's headless backend, but a future contributor may not know this. The lib.rs already explains; tauri.conf.json itself does not.

**Fix:**
Add a JSON comment-equivalent (a `_comment` field, or an entry in `lib.rs` referencing this file):
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "_comment": "Phase 1: no UI windows. Phase 2 will add the popup overlay window.",
  "productName": "Hallmark",
  ...
}
```
Or move the explanation into `lib.rs` (already mostly there).

---

_Reviewed: 2026-05-08_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
