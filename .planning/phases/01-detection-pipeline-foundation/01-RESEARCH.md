# Phase 1: Detection Pipeline Foundation - Research

**Researched:** 2026-05-08
**Domain:** Rust file watcher pipeline (notify + debouncer) + Goldberg adapter + Steam library/Goldberg path discovery + SQLite persistence — all running inside a Tauri v2 Rust backend, no UI in this phase
**Confidence:** HIGH

## Summary

Phase 1 stands up the Tauri v2 + Rust skeleton (no frontend yet) and implements the detection pipeline end-to-end against Goldberg-emulated games. The pipeline reads from disk, debounces multi-fire events, diffs against an in-memory baseline (seeded at startup so historic unlocks never spam), enriches with cross-source dedup, and writes events both to SQLite and to a CLI test sink that the planner-defined success criteria target.

The work decomposes into six load-bearing concerns: (1) project skeleton (Cargo workspace, Tauri scaffold, tracing logger, no frontend); (2) `SourceAdapter` trait + types (`RawUnlockEvent`, `SourceKind`); (3) Goldberg adapter (state-file diffing, supports both `Goldberg SteamEmu Saves\` and `GSE Saves\` roots); (4) path discovery (Steam install registry, `libraryfolders.vdf` parsing, Goldberg `local_save.txt` redirect resolution); (5) Watcher Core (notify-debouncer-full at 500ms + cross-source dedup window); (6) SQLite store (single `hallmark.db`, `unlock_history` and `sessions` tables). The phase explicitly defers schema-fetching, popup, and Steam-legit binary VDF — those are Phase 2 / 3.

**Primary recommendation:** Build with notify-debouncer-full 0.7.0's `new_debouncer(Duration::from_millis(500), None, tx)` as the unified watcher entry point — it gives both the 500ms debounce (REQ DETECT-06) and rename tracking for free, removing the need to hand-roll a per-file timer. The Goldberg adapter holds an in-memory `HashMap<(app_id, ach_api_name), bool>` per-source baseline seeded from disk on `start()` BEFORE attaching the watcher (REQ DETECT-05). Cross-source dedup (REQ DETECT-07) is a separate post-debounce stage keyed on `(app_id, ach_api_name)` with a configurable session-scoped TTL (default 10 seconds) so that two adapters firing for the same logical unlock collapse to one event regardless of arrival order.

<user_constraints>
## User Constraints (from CONTEXT.md)

**No CONTEXT.md exists for this phase.** The orchestrator did not invoke `/gsd-discuss-phase` first. Constraints below are derived directly from project-level decisions (PROJECT.md Key Decisions, ROADMAP.md, CLAUDE.md).

### Locked Decisions (from project docs)

- **Tauri v2 + Rust** is the stack — no Electron, no WPF, no native Win32. (PROJECT.md Key Decisions; STACK.md.)
- **File watcher only** — no Steam Web API, no IPC into Steam client, for v1. (PROJECT.md Constraints.)
- **Goldberg-first** in Phase 1 — Steam legit, CreamAPI, SmartSteamEmu are Phase 3. Do not implement them now. (ROADMAP.md Phase 1; STATE.md Recent decisions.)
- **Backend-only** in this phase — no popup UI, no companion window. The Phase 1 deliverable is a CLI test harness that prints unlock events to stdout. (ROADMAP.md Phase 1 Success Criteria #1.)
- **Local-only** — no telemetry, no cloud, no accounts. (PROJECT.md Constraints.)
- **Windows-only v1** — `cfg(target_os = "windows")` is acceptable; cross-platform abstraction is explicitly out of scope. (PROJECT.md Constraints.)
- **Open-source on GitHub** — license + repo hygiene must be set up day one. (PROJECT.md Constraints.)
- **Hobby pace, polish over speed** — prefer reliable patterns over clever ones. (PROJECT.md Constraints.)
- **GSD workflow enforcement** — file edits go through GSD commands. (CLAUDE.md.)

### Claude's Discretion

- Module layout inside `src-tauri/src/` (the structure in ARCHITECTURE.md is a recommendation, not a constraint).
- Choice of error library (anyhow for application errors, thiserror for library errors — both are idiomatic).
- Logging library (tracing + tracing-subscriber recommended; std `log` is acceptable but tracing is preferred for span-aware async work).
- CLI test harness shape — argv-based, separate `bin` target, or `cargo run --example` are all acceptable.
- Whether `unlock_history` records every observed unlock or only those that fire popups (the table needs `notified` boolean either way; in this phase no popup exists, so all rows have `notified = 0`).
- Whether the `sessions` table is populated in this phase — Phase 1 has no game-launch detection, so a single placeholder "global" session can be used or the column nullable. Recommend nullable for now.

### Deferred Ideas (OUT OF SCOPE for Phase 1)

- Popup window, signature sound, animation — Phase 2.
- Companion window, achievement list UI — Phase 2.
- Game-launch / process detection (sysinfo) — Phase 2.
- Schema fetching (Steam Web API `GetSchemaForGame`) and icon caching — Phase 2.
- Steam-legit binary VDF parser, CreamAPI adapter, SmartSteamEmu adapter — Phase 3.
- DND mode, streamer mode, multi-monitor positioning — Phase 4.
- NSIS installer, auto-updater, GitHub Actions release — Phase 4.
- "Fire test popup" tray button — Phase 4 (no popup yet to fire).
- First-run path-discovery wizard UI — Phase 4 (path discovery itself is Phase 1 but exposed only via logs in this phase).
- Theme presets, sound customization, settings UI — explicitly OUT-OF-SCOPE for v1 entirely.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DETECT-01 | Real-time watcher detects unlocks from Goldberg SteamEmu output, including default `%APPDATA%\Goldberg SteamEmu Saves\` and `local_save.txt` redirected paths | Goldberg state file format documented (see Goldberg State File Schema below). Watch strategy: recursive watch of parent directories. `local_save.txt` resolution algorithm specified in Path Discovery section. |
| DETECT-05 | First-launch state seeding — baseline existing achievement state from disk before attaching change handlers (no install-time spam of historic unlocks) | Adapter `start()` reads all current state files into in-memory `HashMap<(appid, ach_api_name), bool>` BEFORE registering with the watcher. Pitfall #1 (PITFALLS.md) and Pitfall #15 (`unlock_time = 0`) drive the algorithm: use the `earned` boolean transition false→true, never the timestamp, as the unlock signal. |
| DETECT-06 | 500ms debounce + content-hash equality check on file events (no double-popups for a single logical write) | `notify-debouncer-full 0.7.0` provides 500ms debounce natively via `new_debouncer(Duration::from_millis(500), None, tx)`. Content-hash equality is implemented as a per-file `last_content_hash: HashMap<PathBuf, [u8; 32]>` (sha2 of file bytes); skip diff if hash unchanged. Both layers required — debounce collapses event bursts, hash check collapses identical writes (Steam emulators write same state multiple times). |
| DETECT-07 | Cross-source duplicate suppression — one logical unlock observed by multiple adapters produces exactly one popup | Post-debounce dedup stage keyed on `(app_id, ach_api_name)`. When an event arrives, check an in-memory `HashSet<(app_id, ach_api_name)>` with TTL (default 10s session-scoped); if present, drop. Also persists to `unlock_history` table with UNIQUE INDEX on `(app_id, ach_api_name, session_id)` as a belt-and-suspenders second line. |
| DETECT-08 | Path discovery — parse Steam `libraryfolders.vdf` (post-2022 location and legacy location) and discover Goldberg redirects via `local_save.txt` adjacent to `steam_api.dll` | Steam install path read from registry `HKLM\SOFTWARE\WOW6432Node\Valve\Steam\InstallPath` (with `HKCU\Software\Valve\Steam\SteamPath` fallback). Both `<SteamPath>\config\libraryfolders.vdf` (master, post-2022) and `<SteamPath>\steamapps\libraryfolders.vdf` (replicated, legacy) parsed; `keyvalues-parser 0.2.3` handles VDF text format. Goldberg redirect: walk discovered Steam library `steamapps\common\` directories, find `steam_api.dll`/`steam_api64.dll`, check for sibling `local_save.txt`; if present, treat its path content as additional watch root. |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| File system watching (notify) | Rust backend (Tauri sidecar tasks) | — | OS API is Win32 ReadDirectoryChangesW; only the Rust process can call it. WebView has no FS access. |
| Path discovery (registry, VDF parsing) | Rust backend | — | Win32 registry + crate-based VDF parsing both Rust-only. |
| Goldberg state file parsing | Rust backend | — | serde_json runs in Rust. WebView has no game-file path access. |
| Debounce + cross-source dedup | Rust backend | — | Pure Rust state machine; no UI involvement. |
| Persistence (SQLite) | Rust backend | — | rusqlite is Rust; Tauri commands expose to WebView later, but Phase 1 has no UI. |
| CLI test harness output | Rust backend (stdout) | — | Phase 1 deliverable is `println!`/`tracing::info!` log lines proving events flow. |
| Configuration / settings | (deferred) | — | Phase 1 needs no user-tunable config; defaults are hard-coded. |
| Frontend / UI | (deferred to Phase 2) | — | This phase has no React build, no `dist/`, no `WebviewWindow`s. |

**Tier rule:** If anything in Phase 1 touches the WebView or React, it has been mis-scoped. The Tauri Builder is set up but `setup()` runs only background tokio tasks; `tauri::generate_context!()` is called but no `WebviewWindowBuilder` is invoked. The Tauri shell is present so Phase 2 can attach UI cleanly without restructuring.

## Standard Stack

### Core (verified versions as of 2026-05-08 via crates.io API)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tauri` | 2.11.1 | App framework (Rust backend host) | [VERIFIED: crates.io] Phase 1 only uses `tauri::Builder::default().setup()` to spawn background tasks; no `WebviewWindow` is created. Sets up the project skeleton Phase 2 will extend. |
| `notify` | 8.2.0 | Cross-platform FS watcher (Windows backend = `ReadDirectoryChangesW`) | [VERIFIED: crates.io] Default backend for the entire Rust ecosystem; `RecommendedWatcher` auto-selects per platform. |
| `notify-debouncer-full` | 0.7.0 | 500ms debounce + rename tracking + file ID cache | [VERIFIED: crates.io] **Note: STACK.md cited 0.5; current is 0.7.0.** API: `new_debouncer(Duration::from_millis(500), None, tx)`. |
| `tokio` | 1.52.2 | Async runtime, mpsc channels | [VERIFIED: crates.io] Tauri requires it; adapter trait is async. Use `rt-multi-thread` + `sync` features. |
| `serde` + `serde_json` | 1.0.228 / 1.0.149 | Goldberg `achievements.json` parsing, persistence type derive | [VERIFIED: crates.io] Universal Rust JSON. Goldberg state file is plain JSON — `serde_json::Value` for tolerant parsing. |
| `rusqlite` | 0.39.0 | SQLite via bundled libsqlite3 | [VERIFIED: crates.io] Use `bundled` feature so the Tauri binary ships its own libsqlite3 (no system dep). Connection pooling via a shared `Mutex<Connection>` is fine for a single-process desktop app at this volume. |
| `keyvalues-parser` | 0.2.3 | Text VDF parsing for `libraryfolders.vdf` | [VERIFIED: crates.io] Pure-Rust VDF parser. Phase 1 needs only TEXT VDF (libraryfolders); BINARY VDF is Phase 3. |
| `winreg` | 0.56.0 | Read `HKLM\...\Steam\InstallPath` | [VERIFIED: crates.io] Standard Rust registry crate. |
| `walkdir` | 2.5.0 | Find `steam_api*.dll` under `steamapps\common\` | [VERIFIED: crates.io] Standard recursive directory walk. |
| `sha2` | 0.11.0 | SHA-256 of file contents for content-hash equality (DETECT-06) | [VERIFIED: crates.io] Pitfall #2 in PITFALLS.md mandates per-file hash to suppress identical re-writes. |
| `anyhow` | 1.0.102 | Application error handling | [VERIFIED: crates.io] Standard for binary-level errors. |
| `thiserror` | 2.0.18 | Domain-specific error types in the adapter trait | [VERIFIED: crates.io] Standard for library-level errors. |
| `tracing` + `tracing-subscriber` | 0.1.44 / 0.3.23 | Structured logging (REQ Success Criterion #5: log all discovered paths at startup) | [VERIFIED: crates.io] Span-aware async logging; `tracing_subscriber::fmt()` for stdout. |
| `async-trait` | 0.1.89 | `#[async_trait]` on the SourceAdapter trait | [VERIFIED: crates.io] Required because Rust still has no native async-fn-in-trait stable for dyn-compatible traits. |
| `dirs` | 6.0.0 | `%APPDATA%` resolution (`dirs::data_dir()`) | [VERIFIED: crates.io] Cross-platform but functionally Windows-only here. |
| `uuid` | 1.23.1 | Session ID generation for `unlock_history.session_id` | [VERIFIED: crates.io] `Uuid::new_v4()`. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `keyvalues-serde` | 0.2.3 | Serde-driven VDF parsing | If `libraryfolders.vdf` parsing logic gets repetitive across both old and new schemas, `keyvalues-serde` lets you derive struct definitions. Optional — `keyvalues-parser` raw KV tree is sufficient for the small handful of fields needed. |
| `parking_lot` | latest | Faster `Mutex` than std | Optional. Stick with `std::sync::Mutex` and `tokio::sync::Mutex` unless contention is measured. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| notify-debouncer-full | notify-debouncer-mini | -mini lacks rename tracking + file ID cache; -full is the right choice when content-aware watching matters. [CITED: docs.rs/notify-debouncer-full] |
| rusqlite (bundled) | tauri-plugin-sql (sqlx) | tauri-plugin-sql is a fine choice but binds SQL to Tauri commands; Phase 1 has no frontend, so direct rusqlite calls in Rust are simpler. Phase 2 can wrap with the plugin if frontend reads are needed. |
| Hand-rolled VDF parser | keyvalues-parser | Hand-rolling is small but the multi-format reality (legacy + post-2022) makes a tested parser safer. |
| `std::fs::read_to_string` + `serde_json::from_str` | `tokio::fs` async read | Goldberg state files are tiny (<10KB); blocking reads inside `spawn_blocking` are fine. Avoids `tokio::fs` overhead. |

**Installation (single Cargo.toml block):**
```bash
# Workspace root
cargo install create-tauri-app --locked
# Choose: Rust + Vanilla (no frontend framework selection; we'll add React later in Phase 2)
cargo create-tauri-app hallmark
cd hallmark/src-tauri
cargo add tauri@2.11
cargo add tokio@1.52 --features rt-multi-thread,sync,macros,time,fs
cargo add notify@8.2 notify-debouncer-full@0.7
cargo add serde@1 serde_json@1 --features serde/derive
cargo add rusqlite@0.39 --features bundled
cargo add keyvalues-parser@0.2 winreg@0.56 walkdir@2.5
cargo add sha2@0.11 anyhow@1 thiserror@2 async-trait@0.1
cargo add tracing@0.1 tracing-subscriber@0.3 dirs@6 uuid@1 --features uuid/v4,uuid/serde
```

**Version verification done:** All 16 versions above were queried against `https://crates.io/api/v1/crates/<name>` on 2026-05-08. The notify-debouncer-full version difference vs prior research (0.5 → 0.7) is the most material correction.

## Architecture Patterns

### System Architecture Diagram

```
[DISK]                                        [Phase 1 Rust process]
                                              ┌──────────────────────────────────────────┐
%APPDATA%\Goldberg SteamEmu Saves\<appid>\    │                                          │
%APPDATA%\GSE Saves\<appid>\                  │  [PathDiscovery]                         │
<game-dir>\<save-from-local_save.txt>\        │   - registry: HKLM SteamPath             │
                                              │   - keyvalues-parser: libraryfolders.vdf │
                                              │   - walkdir: steam_api.dll → local_save  │
                                              │   - tracing::info! every discovered path │
        modify event                          │            │ Vec<PathBuf>                │
            │                                 │            ▼                             │
            └────────────────────────────────►│  [WatcherCore]                           │
                                              │   - notify::recommended_watcher          │
                                              │   - notify-debouncer-full(500ms, None)   │
                                              │   - one channel: Vec<DebouncedEvent>     │
                                              │            │                             │
                                              │            ▼                             │
                                              │  [Per-event dispatch]                    │
                                              │   for event.path → matching adapter      │
                                              │            │                             │
                                              │            ▼                             │
                                              │  [GoldbergAdapter.on_file_changed()]     │
                                              │   1. read file (retry on IOError)        │
                                              │   2. sha256 == last_hash? skip           │
                                              │   3. parse JSON → state map              │
                                              │   4. diff vs in-memory baseline          │
                                              │   5. baseline.update()                   │
                                              │   6. emit RawUnlockEvent per transition  │
                                              │            │                             │
                                              │            │ tokio::mpsc                 │
                                              │            ▼                             │
                                              │  [CrossSourceDedup]                      │
                                              │   - HashSet<(app_id, ach_api_name)>      │
                                              │   - TTL: 10s session-scoped              │
                                              │   - drop or pass-through                 │
                                              │            │                             │
                                              │            ├─────────────────┐           │
                                              │            ▼                 ▼           │
                                              │  [SqliteStore]      [CliSink]            │
                                              │   - INSERT INTO     - println! / tracing │
                                              │     unlock_history    one line per event │
                                              └──────────────────────────────────────────┘
                                                                        │
                                                                        ▼
                                                                stdout (CLI test harness)

(Phase 2 will subscribe to RawUnlockEvent via Tauri events; this phase outputs to stdout only.)
```

### Recommended Project Structure

The structure here is a Phase 1 subset of ARCHITECTURE.md's full layout. Modules deferred to later phases are intentionally absent.

```
src-tauri/
├── Cargo.toml
├── tauri.conf.json              # Tauri config — no `windows` array yet (no UI in Phase 1)
└── src/
    ├── main.rs                  # Entry — calls hallmark_lib::run()
    ├── lib.rs                   # Tauri builder + setup hook spawning background tasks
    ├── error.rs                 # thiserror enums (PathDiscoveryError, AdapterError, StoreError)
    ├── paths.rs                 # PathDiscovery: registry read, VDF parse, local_save resolution
    ├── sources/
    │   ├── mod.rs               # SourceAdapter trait, RawUnlockEvent, SourceKind
    │   └── goldberg.rs          # GoldbergAdapter — only adapter in Phase 1
    ├── watcher/
    │   ├── mod.rs               # WatcherCore: notify-debouncer-full driver
    │   └── dedup.rs             # CrossSourceDedup with TTL
    ├── store/
    │   ├── mod.rs               # SqliteStore: connection setup, migrations
    │   ├── migrations/
    │   │   └── 001_initial.sql  # unlock_history + sessions + settings tables
    │   └── queries.rs           # insert_unlock(), record_session(), etc.
    └── bin/
        └── hallmark-cli.rs      # CLI test harness: starts pipeline, prints events
```

**Why a separate `bin/hallmark-cli.rs`:** the Tauri binary is the production target for later phases, but Phase 1's success criteria require a *CLI test harness* that is not the Tauri main. A `bin` target lets `cargo run --bin hallmark-cli` start the pipeline outside the Tauri shell for testing. This avoids needing to start a Tauri WebView during automated tests.

### Pattern 1: Adapter trait with self-managed snapshot state

**What:** Each `SourceAdapter` owns its baseline `HashMap<(u64, String), bool>`. Watcher Core never sees file contents.

**When to use:** Whenever multiple file formats need to flow into a unified event type. Even though Phase 1 has only Goldberg, this trait MUST be defined now so Phase 3 adapters slot in without restructuring.

**Example:**
```rust
// src-tauri/src/sources/mod.rs
// [VERIFIED: pattern is the same shape used by Achievement-Watcher and matches ARCHITECTURE.md spec]

use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct RawUnlockEvent {
    pub app_id: u64,
    pub ach_api_name: String,
    pub timestamp: u64,        // unix seconds; 0 if source did not record (DO NOT use as unlock signal)
    pub source: SourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Goldberg,
    // Phase 3:
    // SteamLegit, CreamApi, SmartSteamEmu,
    // Future:
    // Community(String),
}

#[async_trait::async_trait]
pub trait SourceAdapter: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn kind(&self) -> SourceKind;
    fn watch_paths(&self) -> Vec<PathBuf>;

    /// Seed baseline from disk. MUST run before any watcher event fires (DETECT-05).
    async fn seed_baseline(&self) -> anyhow::Result<()>;

    /// Called by WatcherCore when a debounced event hits a path returned by watch_paths().
    /// Diffs current state against the seeded baseline and emits new unlocks via tx.
    async fn on_file_changed(
        &self,
        path: PathBuf,
        tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()>;
}
```

### Pattern 2: notify-debouncer-full as the single watcher entry point

**What:** Use `new_debouncer(Duration::from_millis(500), None, tx)` once at startup, register all adapter watch paths against it, dispatch events to adapters by path prefix matching.

**When to use:** Whenever multiple files/directories need watching with a uniform debounce policy.

**Example:**
```rust
// src-tauri/src/watcher/mod.rs
// [VERIFIED: docs.rs/notify-debouncer-full pattern + notify-rs llms.txt]

use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use std::time::Duration;
use tokio::sync::mpsc;

pub async fn run_watcher(
    adapters: Vec<Arc<dyn SourceAdapter>>,
    raw_tx: mpsc::Sender<RawUnlockEvent>,
) -> anyhow::Result<()> {
    // CRITICAL: seed baselines FIRST, then create the watcher (DETECT-05)
    for adapter in &adapters {
        adapter.seed_baseline().await?;
        tracing::info!(adapter = %adapter.name(), "Baseline seeded");
    }

    let (notify_tx, mut notify_rx) = mpsc::channel::<DebounceEventResult>(64);

    let mut debouncer = new_debouncer(
        Duration::from_millis(500),  // DETECT-06: 500ms debounce
        None,                        // tick rate auto = 1/4 of timeout
        move |res: DebounceEventResult| {
            // Note: this is a sync closure; bridge to tokio with blocking_send.
            let _ = notify_tx.blocking_send(res);
        },
    )?;

    // Register every adapter's watch paths with the SAME debouncer
    for adapter in &adapters {
        for path in adapter.watch_paths() {
            tracing::info!(adapter = %adapter.name(), ?path, "Watching path");
            debouncer.watch(&path, RecursiveMode::Recursive)?;
        }
    }

    while let Some(res) = notify_rx.recv().await {
        match res {
            Ok(events) => {
                for event in events {
                    for path in &event.event.paths {
                        // Find the adapter whose watch_paths() prefix-matches this path
                        if let Some(adapter) = adapters.iter().find(|a| {
                            a.watch_paths().iter().any(|wp| path.starts_with(wp))
                        }) {
                            if let Err(e) = adapter
                                .on_file_changed(path.clone(), raw_tx.clone())
                                .await
                            {
                                tracing::warn!(?e, ?path, "Adapter error");
                            }
                        }
                    }
                }
            }
            Err(errs) => {
                for e in errs {
                    tracing::warn!(?e, "notify error");
                }
            }
        }
    }
    Ok(())
}
```

**Note on the blocking_send bridge:** notify-debouncer-full's callback is sync (it runs on the debouncer's internal tick thread). Use `mpsc::Sender::blocking_send` from inside the callback; do NOT spawn a tokio runtime there. The callback finishes quickly because it only forwards to a channel.

### Pattern 3: Cross-source dedup as a separate pipeline stage

**What:** A small struct holding `HashMap<(u64, String), Instant>`. On each event arrival, look up the key; if present and within TTL, drop; else insert and forward.

**When to use:** Whenever multiple adapters can observe the same logical event (DETECT-07). In Phase 1 this is exercised only by the simulated cross-source dedup test (Success Criterion #4); in Phase 3 it becomes load-bearing for real users running both legitimate Steam and Goldberg.

**Example:**
```rust
// src-tauri/src/watcher/dedup.rs
// [VERIFIED: standard TTL-cache pattern; this is a hand-rolled minimal implementation]

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct CrossSourceDedup {
    seen: HashMap<(u64, String), Instant>,
    ttl: Duration,
}

impl CrossSourceDedup {
    pub fn new(ttl: Duration) -> Self {
        Self { seen: HashMap::new(), ttl }
    }

    /// Returns true if this event is a duplicate (and should be dropped).
    /// Side effect: sweeps expired entries before checking.
    pub fn is_duplicate(&mut self, app_id: u64, ach_api_name: &str) -> bool {
        let now = Instant::now();
        // O(n) sweep is acceptable; n is small (one session of unlocks).
        self.seen.retain(|_, ts| now.duration_since(*ts) < self.ttl);

        let key = (app_id, ach_api_name.to_string());
        if self.seen.contains_key(&key) {
            true
        } else {
            self.seen.insert(key, now);
            false
        }
    }
}
```

The TTL value (default 10s) is well above the 500ms debounce — the assumption is that two adapters writing for the same logical unlock will both emit within 10s. Real-world simultaneity is sub-second; 10s is a generous safety margin.

### Anti-Patterns to Avoid

- **Hard-coding `%APPDATA%\Goldberg SteamEmu Saves\` as the only watch path.** The modern fork (`gbe_fork`, used by most 2024+ scene releases) writes to `%APPDATA%\GSE Saves\` instead. Watch BOTH default roots; also watch any `local_save.txt`-redirected directories. [CITED: gbe_fork README; see Goldberg State File Schema below]
- **Using `unlock_time` / `earned_time` as the unlock signal.** PITFALLS.md #15: emulators write `0` for unknown timestamps. The `earned: bool` transition false→true is the only reliable signal.
- **Reading the file inside the notify callback thread.** notify's callback is on the watcher thread; FS I/O there blocks future events. Always dispatch to a tokio task or use the debouncer (which already inserts a timed boundary).
- **Mutating the baseline before sending the unlock event.** Update order: read file → diff against baseline → emit events → THEN update baseline. Reversing this loses the diff.
- **Skipping the content-hash check.** Steam emulators write the same JSON 2-3 times per logical unlock with identical content. Without `last_content_hash`, even debouncing isn't enough — a single user write spread across 600ms produces two debounced batches with identical content.
- **Storing baseline only in memory.** PITFALLS.md #1 + Success Criterion #2 both demand: a user closes Hallmark, earns 5 achievements offline, restarts Hallmark — those 5 must NOT pop. The in-memory baseline solves the "during this run" case; the on-disk persistent state solves the cross-restart case. **Phase 1 takes the simpler route: re-seed baseline from current disk state on every startup.** Because the diff fires on transitions only after seed, achievements earned while Hallmark was off are silently absorbed into the new baseline (CORRECT behavior — they aren't "new" relative to Hallmark's knowledge).
- **Watching individual `<appid>` subdirectories.** New games appear without warning. Watch the parent (`%APPDATA%\Goldberg SteamEmu Saves\`) recursively.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 500ms debounce window | Per-file `tokio::time::sleep` timer reset on every event | `notify-debouncer-full` 0.7.0 | Handles rename merging, file ID tracking, and tick scheduling correctly. Hand-rolled timers leak on cancellation and miss rename-as-Both events. |
| File-content equality check | Byte-by-byte `Vec<u8>` comparison | `sha2::Sha256` hash of bytes, store digest | Hashing is faster on re-comparison and the digest is small enough to keep per-file in memory indefinitely. |
| Steam libraryfolders.vdf parser | Hand-rolled VDF tokenizer | `keyvalues-parser` 0.2.3 | Two known formats (pre-2022 flat, post-2022 nested). Hand-rolling needs to handle quoting, escaping, and comments. |
| Windows registry read | `windows` / `windows-rs` direct API | `winreg` 0.56.0 | `winreg` wraps RegOpenKeyEx + RegQueryValueEx with idiomatic Rust. The bare windows-rs is needed for Phase 2 HWND work; Phase 1 doesn't need it. |
| `%APPDATA%` resolution | `std::env::var("APPDATA")` | `dirs::data_dir()` | `dirs` handles the `Roaming` vs `Local` distinction explicitly and is the de-facto standard. |
| SQLite migrations | Hand-rolled "if table not exists" SQL | rusqlite + a numbered .sql files convention | A single `001_initial.sql` is enough for Phase 1; any in-tree migration runner (e.g. `refinery` or just a hand-loop over files) is overkill at this scale. Just embed the SQL with `include_str!` and execute it idempotently with `CREATE TABLE IF NOT EXISTS`. |
| File read with retry on `IOError` | None: just retry | small loop: 3 attempts × 50ms sleep | PITFALLS.md #3 (file locked / partial read). Hand-roll this — it's a 5-line function. The `retry` crate is overkill. |
| UUID for session_id | random String | `uuid::Uuid::new_v4()` | Phase 1's "session" is essentially the program lifetime; generate one at startup. |

**Key insight:** the only "build it yourself" piece is the dedup `HashMap<(u64, String), Instant>` and the per-adapter baseline `HashMap<(u64, String), bool>`. Everything else is pulled in.

## Runtime State Inventory

> Phase 1 is a greenfield phase that *creates* the project. There are no pre-existing runtime systems to migrate. This section is included for completeness with explicit "None" answers.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no prior database exists. The project repository contains only `CLAUDE.md` and `.planning/` artifacts. The `hallmark.db` SQLite file will be created fresh by Phase 1. | None — first-run creation |
| Live service config | None — no external services (Datadog, n8n, Cloudflare, etc.) are part of this app. The app reads other-vendor state files (Goldberg `achievements.json`) but does not own any live service config. | None |
| OS-registered state | None YET — Phase 4 will register a `HKCU\...\Run` entry for start-with-Windows; Phase 1 does not. | None in Phase 1 |
| Secrets/env vars | None — the app uses no API keys, no auth, no env-injected secrets. (Steam Web API is keyless and is not used until Phase 2.) | None |
| Build artifacts / installed packages | None — there is no prior compiled artifact, no prior `target/` directory, no prior installer. The phase begins from a literally empty `src-tauri/` after `cargo create-tauri-app`. | None |

**Verified by:** `ls -la C:/Users/reema/Documents/Programming/achievements/` shows only `.claude/`, `.git/`, `.planning/`, and `CLAUDE.md`. No `src-tauri/`, no `package.json`, no `Cargo.toml`.

## Common Pitfalls

### Pitfall 1: First-launch state seeding (REQ DETECT-05 — verifying Success Criterion #2)

**What goes wrong:** User installs Hallmark; their Goldberg save directory already has 200 earned achievements; on first launch the watcher fires events for all 200, dumping logs (and in Phase 2, popups) for unlocks the user got months ago.

**Why it happens:** The naive watcher path is "on file change, parse file, emit any `earned:true` as an unlock event." The fix is a baseline read at startup BEFORE attaching the watcher.

**How to avoid:**
1. In `GoldbergAdapter::seed_baseline()`, walk every `<appid>\achievements.json` under both `Goldberg SteamEmu Saves\` and `GSE Saves\`.
2. For each file, parse and populate `self.baseline.write().await.insert((app_id, ach_api_name), earned_bool)` for every achievement (earned AND unearned alike — full snapshot).
3. ONLY AFTER `seed_baseline()` returns successfully for all adapters, register paths with the watcher.
4. In `on_file_changed()`: emit unlock events ONLY for keys whose stored value flipped `false → true`. Same-value (`true → true`) is silent. New keys (game just got achievements added) where current value is `true` is treated as already-earned (silent — no spam) — this matches user expectation when they install a new game with cracked save.

**Warning signs:**
- Test Criterion #2 fails: dropping a populated dir before launch produces popups on launch.
- Logs show "emit unlock_event" before "Baseline seeded" entries.

### Pitfall 2: notify-debouncer-full 0.5 vs 0.7 API drift

**What goes wrong:** Following STACK.md's `notify-debouncer-full = "0.5"` instruction picks up a version with a different API signature.

**Why it happens:** [VERIFIED: crates.io API on 2026-05-08] Current stable is **0.7.0**. STACK.md's "0.5" was the cached information at research time but the crate moved.

**How to avoid:** Pin `notify-debouncer-full = "0.7"` in Cargo.toml. The 0.7 API used in the example above is correct. Do NOT use the version in STACK.md.

### Pitfall 3: Goldberg path realities — TWO save roots, plus redirects

**What goes wrong:** Watching only `%APPDATA%\Goldberg SteamEmu Saves\` misses every `gbe_fork`-released game (most 2024+ cracks), which writes to `%APPDATA%\GSE Saves\`. Also misses `local_save.txt`-redirected paths.

**Why it happens:** [CITED: WebSearch confirmed both paths in use as of 2025/2026; achievement-watchdog GitHub] The original Goldberg used the long name; the most active fork (`Detanup01/gbe_fork`) uses the shorter name. Both are widely deployed.

**How to avoid:** The Goldberg adapter's `watch_paths()` returns up to FOUR roots:
1. `%APPDATA%\Goldberg SteamEmu Saves\` (legacy default)
2. `%APPDATA%\GSE Saves\` (gbe_fork default)
3. `%PUBLIC%\Documents\Goldberg SteamEmu Saves\` (rare; older releases)
4. Every `local_save.txt`-resolved path (variable count; computed at startup)

Filter for existence — only watch directories that exist at startup. Re-running discovery on a long-running session is a Phase 4 concern (first-run wizard); Phase 1 discovers once at start, logs results, sticks with them.

**Warning signs:** Test Criterion #3 fails (a game with `local_save.txt` produces no events).

### Pitfall 4: Confusing the SCHEMA file with the STATE file

**What goes wrong:** The Goldberg `steam_settings\achievements.json` (alongside `steam_api.dll` in the game's directory) is the SCHEMA — array of `{name, displayName, description, icon, hidden}` objects. The `%APPDATA%\Goldberg SteamEmu Saves\<appid>\achievements.json` is the STATE — object map of `{ach_api_name: {earned, earned_time}}`. Parsing one as the other returns garbage.

**Why it happens:** Same filename in two different locations. PITFALLS.md mentions the path; this RESEARCH.md is the first place that documents the schema vs state distinction explicitly.

**How to avoid:**
- Phase 1 ONLY reads the STATE file (`%APPDATA%\...`).
- The SCHEMA file (alongside steam_api.dll) is untouched in Phase 1; Phase 2 uses it for offline-fallback display name resolution.

**Warning signs:** Parser errors trying to read `achievements.json` as an object when it's an array.

### Pitfall 5: notify recursive watch on a non-existent directory

**What goes wrong:** Calling `debouncer.watch(path, RecursiveMode::Recursive)` on a path that doesn't exist returns `notify::ErrorKind::PathNotFound`. If the user has Goldberg installed but never ran a game, `Goldberg SteamEmu Saves\` may not exist yet.

**Why it happens:** `ReadDirectoryChangesW` requires an existing directory handle.

**How to avoid:**
- Filter `watch_paths()` against `path.exists()` before registering.
- For paths that don't exist at startup but might appear later (user installs a new game), Phase 1 logs the absence and accepts the limitation — the user must restart Hallmark after the directory first appears. Phase 4's first-run wizard surfaces this. (A more robust solution — watching the parent of the parent — is over-scoped for v1.)

**Warning signs:** Startup error like `Watch error: PathNotFound for "C:\\Users\\X\\AppData\\Roaming\\Goldberg SteamEmu Saves"`.

### Pitfall 6: notify InternalBufferSize on Windows for many directories

**What goes wrong:** `ReadDirectoryChangesW` has a default 8KB internal buffer. Recursive watch on a directory with many sub-trees (200+ game appids, each writing) overflows the buffer; events are silently dropped with an `ERROR_NOTIFY_ENUM_DIR` from the OS.

**Why it happens:** [CITED: notify-rs docs.rs]; PITFALLS.md integration gotcha #1.

**How to avoid:** notify 8.x exposes `Config::with_buffer_size` on `Config` for backends that support it (the Windows backend does). Set buffer to 65536 (64KB) when constructing the watcher via `new_debouncer_opt`. For Phase 1's Goldberg-only use case the default is fine — but document the knob for Phase 3 when Steam-legit recursive watches expand the tree.

### Pitfall 7: Unicode paths and `dirs::data_dir()`

**What goes wrong:** A Windows username with non-ASCII characters (e.g., Cyrillic, Japanese) makes `%APPDATA%` resolve to a path that some crates handle correctly and some don't. Logging the path is fine; passing it through `Path::display()` is fine; using it in error messages is fine. Nothing in Phase 1's stack is known to break on Unicode paths, but every path operation should use `Path`/`PathBuf`, never `&str`.

**How to avoid:** Standard hygiene. Avoid `path.to_str().unwrap()`; prefer `path.display()` for logs, pass `&Path` everywhere internally.

## Code Examples

Verified patterns referenced from the official sources noted.

### Goldberg state file parse + diff (the heart of the adapter)

```rust
// src-tauri/src/sources/goldberg.rs
// [CITED: WebSearch — Goldberg state schema confirmed 2026-05-08]
// State file shape: { "ACH_API_NAME": { "earned": true|false, "earned_time": 1700000000 } }

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde::Deserialize;
use tokio::sync::{mpsc, RwLock};

#[derive(Deserialize, Debug)]
struct GoldbergEntry {
    earned: bool,
    #[serde(default)]
    earned_time: u64,  // may be 0 or missing — DO NOT use as unlock signal
}

type StateMap = HashMap<String, bool>;  // ach_api_name -> earned

pub struct GoldbergAdapter {
    /// Watch roots discovered at startup.
    roots: Vec<PathBuf>,
    /// In-memory baseline. Key: (app_id, ach_api_name). Value: earned.
    baseline: Arc<RwLock<HashMap<(u64, String), bool>>>,
    /// Per-file content hash to suppress identical re-writes (DETECT-06).
    last_hash: Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>,
}

impl GoldbergAdapter {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self {
            roots,
            baseline: Arc::new(RwLock::new(HashMap::new())),
            last_hash: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Resolve an <appid>\achievements.json file to (app_id: u64).
    /// Path shape: <root>/<appid>/achievements.json
    fn extract_app_id(&self, path: &std::path::Path) -> Option<u64> {
        path.parent()?
            .file_name()?
            .to_str()?
            .parse::<u64>()
            .ok()
    }

    fn parse_state(json: &str) -> anyhow::Result<StateMap> {
        let raw: HashMap<String, GoldbergEntry> = serde_json::from_str(json)?;
        Ok(raw.into_iter().map(|(k, v)| (k, v.earned)).collect())
    }
}

#[async_trait::async_trait]
impl SourceAdapter for GoldbergAdapter {
    fn name(&self) -> &str { "goldberg" }
    fn kind(&self) -> SourceKind { SourceKind::Goldberg }
    fn watch_paths(&self) -> Vec<PathBuf> { self.roots.clone() }

    async fn seed_baseline(&self) -> anyhow::Result<()> {
        let mut baseline = self.baseline.write().await;
        for root in &self.roots {
            if !root.exists() { continue; }
            for entry in walkdir::WalkDir::new(root).max_depth(2) {
                let entry = entry?;
                if entry.file_name() != "achievements.json" { continue; }
                let path = entry.path().to_path_buf();
                let Some(app_id) = self.extract_app_id(&path) else { continue; };
                let json = match read_with_retry(&path) {
                    Ok(s) => s,
                    Err(e) => { tracing::warn!(?path, ?e, "seed: read failed, skip"); continue; }
                };
                let state = Self::parse_state(&json).unwrap_or_default();
                for (api_name, earned) in state {
                    baseline.insert((app_id, api_name), earned);
                }
            }
        }
        tracing::info!(entries = baseline.len(), "Goldberg baseline seeded");
        Ok(())
    }

    async fn on_file_changed(
        &self,
        path: PathBuf,
        tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()> {
        if path.file_name().and_then(|n| n.to_str()) != Some("achievements.json") {
            return Ok(());
        }
        let Some(app_id) = self.extract_app_id(&path) else {
            return Ok(());
        };

        // Read with retry (PITFALLS.md #3)
        let json = read_with_retry(&path)?;

        // Content-hash equality (DETECT-06)
        use sha2::{Digest, Sha256};
        let hash: [u8; 32] = Sha256::digest(json.as_bytes()).into();
        {
            let mut hashes = self.last_hash.write().await;
            if hashes.get(&path) == Some(&hash) {
                return Ok(());  // identical content; skip
            }
            hashes.insert(path.clone(), hash);
        }

        let state = Self::parse_state(&json)?;

        // Diff against baseline. Lock once; iterate.
        let mut baseline = self.baseline.write().await;
        for (api_name, earned_now) in state {
            let key = (app_id, api_name.clone());
            let was = baseline.get(&key).copied().unwrap_or(false);
            if !was && earned_now {
                // Transition: emit unlock event
                let evt = RawUnlockEvent {
                    app_id,
                    ach_api_name: api_name,
                    timestamp: 0,  // we DO NOT trust earned_time as a freshness signal; downstream stamps wall clock
                    source: SourceKind::Goldberg,
                };
                if tx.send(evt).await.is_err() {
                    tracing::error!("RawUnlockEvent receiver dropped");
                }
            }
            baseline.insert(key, earned_now);
        }
        Ok(())
    }
}

fn read_with_retry(path: &std::path::Path) -> anyhow::Result<String> {
    use std::time::Duration;
    let mut last_err = None;
    for _ in 0..3 {
        match std::fs::read_to_string(path) {
            Ok(s) => return Ok(s),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied
                  || e.raw_os_error() == Some(32) /* sharing violation */ => {
                last_err = Some(e);
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(e.into()),
        }
    }
    Err(last_err.unwrap().into())
}
```

### Path discovery: Steam install + libraryfolders.vdf + Goldberg local_save

```rust
// src-tauri/src/paths.rs
// [VERIFIED: WebSearch — both registry locations and both VDF locations confirmed]

use std::path::PathBuf;
use winreg::enums::*;
use winreg::RegKey;

pub struct DiscoveredPaths {
    pub steam_install: Option<PathBuf>,
    pub steam_libraries: Vec<PathBuf>,            // Each is a "Steam library root" (contains steamapps/)
    pub goldberg_save_roots: Vec<PathBuf>,         // Default Goldberg/GSE Saves dirs that exist
    pub goldberg_local_save_redirects: Vec<PathBuf>,  // Resolved from local_save.txt next to steam_api*.dll
}

pub fn discover() -> DiscoveredPaths {
    let steam_install = read_steam_install();
    let steam_libraries = steam_install
        .as_ref()
        .map(|p| parse_libraryfolders(p))
        .unwrap_or_default();

    let goldberg_save_roots = goldberg_default_roots();
    let goldberg_local_save_redirects = scan_local_save_redirects(&steam_libraries);

    let result = DiscoveredPaths {
        steam_install,
        steam_libraries,
        goldberg_save_roots,
        goldberg_local_save_redirects,
    };

    // Success Criterion #5: log all discovered paths at startup
    tracing::info!(?result.steam_install, "Steam install path");
    for p in &result.steam_libraries {
        tracing::info!(?p, "Steam library");
    }
    for p in &result.goldberg_save_roots {
        tracing::info!(?p, "Goldberg save root (default)");
    }
    for p in &result.goldberg_local_save_redirects {
        tracing::info!(?p, "Goldberg local_save.txt redirect");
    }
    if result.steam_install.is_none() {
        tracing::warn!("Steam install not detected — no library scanning possible");
    }

    result
}

fn read_steam_install() -> Option<PathBuf> {
    // Try HKLM\SOFTWARE\WOW6432Node\Valve\Steam first (64-bit user; most users)
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey(r"SOFTWARE\WOW6432Node\Valve\Steam") {
        if let Ok(p) = key.get_value::<String, _>("InstallPath") {
            return Some(PathBuf::from(p));
        }
    }
    // Fallback: HKCU\Software\Valve\Steam (current-user install or 32-bit)
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(r"Software\Valve\Steam") {
        if let Ok(p) = key.get_value::<String, _>("SteamPath") {
            return Some(PathBuf::from(p));
        }
    }
    None
}

fn parse_libraryfolders(steam_install: &std::path::Path) -> Vec<PathBuf> {
    // Try post-2022 location first; fall back to legacy
    let candidates = [
        steam_install.join("config").join("libraryfolders.vdf"),
        steam_install.join("steamapps").join("libraryfolders.vdf"),
    ];
    for path in &candidates {
        if !path.exists() { continue; }
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => { tracing::warn!(?path, ?e, "vdf read failed"); continue; }
        };
        // Hand-extract "path" values; both old and new formats put paths inside double-quoted "path" tokens
        // For a more robust parser use keyvalues_parser::Vdf::parse(&text)
        let mut libs = Vec::new();
        // The post-2022 schema is:
        // "libraryfolders" { "0" { "path" "C:\\..." ... } "1" { "path" "D:\\..." ... } }
        // Use keyvalues-parser for the non-prototype version.
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with("\"path\"") {
                if let Some(start) = line[6..].find('"').map(|i| i + 7) {
                    if let Some(end) = line[start..].find('"').map(|i| i + start) {
                        libs.push(PathBuf::from(line[start..end].replace("\\\\", "\\")));
                    }
                }
            }
        }
        // Steam install itself is always a library
        if !libs.iter().any(|l| l == steam_install) {
            libs.insert(0, steam_install.to_path_buf());
        }
        return libs;
    }
    // No VDF found; just use the install root
    vec![steam_install.to_path_buf()]
}

fn goldberg_default_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(appdata) = dirs::data_dir() {
        let candidates = [
            appdata.join("Goldberg SteamEmu Saves"),
            appdata.join("GSE Saves"),
        ];
        for p in candidates {
            if p.exists() { roots.push(p); }
        }
    }
    if let Some(public) = std::env::var_os("PUBLIC") {
        let p = PathBuf::from(public).join("Documents").join("Goldberg SteamEmu Saves");
        if p.exists() { roots.push(p); }
    }
    roots
}

fn scan_local_save_redirects(libraries: &[PathBuf]) -> Vec<PathBuf> {
    let mut redirects = Vec::new();
    for lib in libraries {
        let common = lib.join("steamapps").join("common");
        if !common.exists() { continue; }
        for entry in walkdir::WalkDir::new(&common).max_depth(8) {
            let Ok(entry) = entry else { continue; };
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name == "steam_api.dll" || name == "steam_api64.dll" {
                let dir = entry.path().parent().unwrap();
                let local_save = dir.join("local_save.txt");
                if !local_save.exists() { continue; }
                let Ok(rel) = std::fs::read_to_string(&local_save) else { continue; };
                let rel = rel.trim();
                let resolved = if std::path::Path::new(rel).is_absolute() {
                    PathBuf::from(rel)
                } else {
                    dir.join(rel)
                };
                if resolved.exists() {
                    redirects.push(resolved);
                }
            }
        }
    }
    redirects
}
```

**Note on `parse_libraryfolders`:** The example uses a hand-rolled scan for brevity. The phase plan should pick `keyvalues-parser` to get the same behaviour with proper VDF handling — the manual line scan is shown only to illustrate the data shape.

### SQLite migration + insert

```rust
// src-tauri/src/store/migrations/001_initial.sql
// [VERIFIED: ARCHITECTURE.md schema, simplified for Phase 1 — schema_cache and icon_cache deferred to Phase 2]

CREATE TABLE IF NOT EXISTS unlock_history (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    app_id        INTEGER NOT NULL,
    ach_api_name  TEXT    NOT NULL,
    source        TEXT    NOT NULL,
    unlocked_at   INTEGER NOT NULL,
    session_id    TEXT,
    notified      INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_unlock_session ON unlock_history(session_id);
CREATE INDEX IF NOT EXISTS idx_unlock_app     ON unlock_history(app_id, ach_api_name);
CREATE UNIQUE INDEX IF NOT EXISTS idx_unlock_dedup
    ON unlock_history(app_id, ach_api_name, session_id);

CREATE TABLE IF NOT EXISTS sessions (
    session_id    TEXT    PRIMARY KEY,
    app_id        INTEGER,
    started_at    INTEGER NOT NULL,
    ended_at      INTEGER
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

```rust
// src-tauri/src/store/mod.rs
// [CITED: rusqlite docs]

use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;

pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    pub fn open(db_path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(include_str!("migrations/001_initial.sql"))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn record_unlock(
        &self,
        app_id: u64,
        ach_api_name: &str,
        source: &str,
        session_id: Option<&str>,
    ) -> anyhow::Result<bool> {
        // Returns true if inserted (new), false if dedup-rejected.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        let conn = self.conn.lock().unwrap();
        let res = conn.execute(
            "INSERT OR IGNORE INTO unlock_history
                (app_id, ach_api_name, source, unlocked_at, session_id, notified)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![app_id as i64, ach_api_name, source, now, session_id],
        )?;
        Ok(res == 1)
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `notify-debouncer-full = "0.5"` | `notify-debouncer-full = "0.7.0"` | 2025+ | API for `new_debouncer` is the same shape, but version bump should be picked up; STACK.md is stale here. |
| Steam `libraryfolders.vdf` in `<Steam>\steamapps\` only | Steam `libraryfolders.vdf` master in `<Steam>\config\`, replicated to `steamapps\` | Mid-2022 | Both must be checked; `config\` first (master), `steamapps\` as legacy fallback. |
| Goldberg saves only at `%APPDATA%\Goldberg SteamEmu Saves\` | Two roots: `Goldberg SteamEmu Saves\` AND `GSE Saves\` (gbe_fork) | 2023+ | Watching only the legacy root misses the majority of 2024+ scene releases. |
| `unlock_time > 0` as the unlock signal | `earned: true` boolean transition (false→true) | Always was correct | Goldberg state file uses `earned_time` (not `unlock_time`); both names cover the same concept and both can be `0` for "earned but timestamp unknown." |

**Deprecated/outdated:**
- Polling-based custom debouncers (per-file timer, per-event reschedule) — superseded by `notify-debouncer-full`.
- Hand-rolled VDF parsers — `keyvalues-parser` is mature.
- `winapi` crate — superseded by `windows`/`windows-rs` for FFI. `winreg` 0.56 already wraps cleanly.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The cross-source dedup TTL of 10 seconds is sufficient — no two real adapters will emit the same logical unlock more than 10s apart. | Cross-source dedup pattern | If an adapter takes longer (e.g., delayed FS event on a slow disk), a duplicate slips through. Belt-and-suspenders: SQLite UNIQUE INDEX on `(app_id, ach_api_name, session_id)` catches what TTL misses. Mitigation already in place. |
| A2 | The Goldberg state file is small enough (< 100 KB typical, < 1 MB pathological) that synchronous `std::fs::read_to_string` is acceptable inside the `on_file_changed` async path. | Goldberg adapter implementation | Wrong only on extreme pathological inputs; an adversarial 100MB JSON would block the event loop briefly. Mitigation: optionally wrap in `tokio::task::spawn_blocking` if profiling shows blocking. |
| A3 | The user's Windows `%APPDATA%` path is writable and `dirs::data_dir()` resolves correctly. | Path discovery | Standard for all Windows desktop apps; only fails on locked-down corporate machines, which are out of scope. |
| A4 | `gbe_fork`'s state file format is identical to legacy Goldberg's state file format (same field names, same shape). | Pitfall #4 / Goldberg State File Schema | If gbe_fork diverged (e.g., used `unlocked` instead of `earned`), the parser drops every entry as `Default::default()` (false) and emits no events. **Mitigation: Phase 1 plan should include a one-time empirical inspection of a real `GSE Saves\` state file before locking the adapter design. WebSearch found one report consistent with the legacy schema but no canonical doc.** [LOW confidence on this specific point — flag for plan-time validation.] |
| A5 | `UNIQUE INDEX (app_id, ach_api_name, session_id)` with `INSERT OR IGNORE` is sufficient cross-restart dedup, given each Hallmark startup generates a fresh `session_id`. | SQLite schema | This means cross-RESTART dedup is NOT enforced by the DB — a user closes Hallmark, achievement persists in `unlock_history`, restarts, baseline re-seeds (silent), and the achievement does NOT re-fire. Verified by the baseline-re-seed logic. The DB-level UNIQUE is only for in-session dedup. This is correct; documenting the assumption explicitly. |
| A6 | The Phase 1 success criteria can be exercised without actually installing Goldberg — by hand-creating fixture files in a test directory. | Test strategy | Implies the CLI test harness accepts an `--override-goldberg-root <path>` flag (or env var) so tests don't pollute real `%APPDATA%`. The phase plan should include such a flag. |

**If any of A1–A6 is wrong:** Phase 1 may still ship correctly because each has a defensive fallback or a plan-phase validation hook. The most material is A4 (gbe_fork schema parity) — the planner should add a "Wave 0: empirical Goldberg state file inspection" step to confirm field names against a real save.

## Open Questions (RESOLVED)

1. **gbe_fork state file field names** (relates to A4 above)
   - What we know: WebSearch confirmed `earned` and `earned_time` for legacy Goldberg; gbe_fork is an active fork.
   - What's unclear: Whether gbe_fork preserves the same field names. The fork's own README does not document the runtime state schema explicitly.
   - Recommendation: First task of Phase 1 implementation should be to run a real Goldberg-or-gbe_fork-cracked indie game once, then `cat` the resulting state file and confirm field names. If divergent, parameterize `GoldbergEntry` field names per-adapter-variant. **Plan should include this empirical check before the adapter code is finalized.**
   - **RESOLVED:** Plan 01 Task 1 produces `empirical-goldberg-schema-NOTES.md` resolving Assumption A4 (empirical inspection or documented fallback). Plan 04 Task 1 locks the parser to `{ "ACH_NAME": { "earned": bool, "earned_time": u64 } }` with `#[serde(default)]` on `earned_time` per the NOTES.md decision.

2. **PUBLIC documents path for Goldberg saves**
   - What we know: Some older guides mention `%PUBLIC%\Documents\Goldberg SteamEmu Saves\`.
   - What's unclear: Whether modern Goldberg/gbe_fork ever writes there.
   - Recommendation: Include the path in `goldberg_default_roots()` (cheap to check `path.exists()`), don't add as a primary path. If absent it's silently skipped.
   - **RESOLVED:** Plan 03 Task 2 `goldberg_default_roots()` includes `%PUBLIC%\Documents\Goldberg SteamEmu Saves\` as the third candidate path; existence-filtered so absent paths do not produce noise.

3. **Whether to use `tokio::sync::RwLock` or `std::sync::RwLock` for the baseline**
   - What we know: The baseline is read-rare, write-on-update. Lock contention is negligible.
   - What's unclear: Whether the await-points across the lock matter.
   - Recommendation: Use `tokio::sync::RwLock` because `seed_baseline()` and `on_file_changed()` are async; mixing std and tokio locks is a footgun. The async lock is also lighter on this workload.
   - **RESOLVED:** Plan 04 Task 1 `GoldbergAdapter` uses `Arc<tokio::sync::RwLock<HashMap<(u64, String), bool>>>` for the baseline and `Arc<tokio::sync::RwLock<HashMap<PathBuf, [u8; 32]>>>` for last_hash, per the recommendation.

4. **Test harness shape: `cargo run --bin hallmark-cli` vs. `cargo test`**
   - What we know: Success criteria are stated as observable behaviors (one event per drop, zero events on populated init, etc.). They can be checked manually OR via Rust integration tests.
   - What's unclear: Whether the planner wants automated tests or a manual CLI verification protocol.
   - Recommendation: Both. The `bin/hallmark-cli` for manual exercise; `tests/integration_phase1.rs` for automated success-criterion-style tests against fixture directories. Since `nyquist_validation = false`, automated tests are not contractually required, but the success criteria are precise enough to be cheaply automated.
   - **RESOLVED:** Both shipped. Plan 05 Task 2 produces `src-tauri/src/bin/hallmark-cli.rs` (manual harness) AND Plan 05 Task 3 produces `src-tauri/tests/integration_phase1.rs` (5 automated SC tests, one per ROADMAP success criterion).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All Rust compilation | Unknown — assumed | (need 1.85+) | Install via `rustup` |
| `cargo` | Build | Unknown — assumed | (bundled with rustup) | — |
| `cargo-tauri` CLI | `cargo tauri dev/build` | Unknown — likely missing | — | `cargo install tauri-cli --version "^2"` |
| Node.js + npm | Frontend build (Phase 2 onward) | Unknown — assumed | — | not needed in Phase 1 if backend-only |
| Windows 10/11 | All testing | Yes (target platform) | win32 / 10.0.26200 (per env block) | — |
| Steam (legitimate install, optional) | Path discovery testing for libraryfolders.vdf | Unknown | — | Skip Steam-discovery tests; Goldberg path tests work without Steam |
| Goldberg-using game (real or fixture) | Manual end-to-end testing | Unknown | — | Fixture: hand-created `<test-root>\<appid>\achievements.json` with sample state |

**Missing dependencies with no fallback:** None — Rust + Cargo are the only hard requirements, and they install cleanly via rustup.

**Missing dependencies with fallback:**
- **No real Steam install available for testing:** The phase plan should include fixture generators that simulate `<SteamPath>\config\libraryfolders.vdf` — the real install is not required for unit tests, only for one-time manual end-to-end validation.
- **No real Goldberg install available for testing:** Same — the CLI test harness should accept overrides for `%APPDATA%` discovery so tests run against fixture directories. Real install only needed for manual confirmation of A4 (gbe_fork field names).

**Verification action (first-task work):**
```powershell
# In the phase plan, the first wave should run:
rustup --version           # Confirm Rust toolchain
cargo --version            # Confirm cargo
where cargo-tauri 2>$null  # Optional — install if missing
node --version 2>$null     # Optional — needed only when Phase 2 starts
```

## Sources

### Primary (HIGH confidence)
- crates.io API (`https://crates.io/api/v1/crates/<name>`) — verified all 16 crate versions on 2026-05-08 (notify 8.2.0, notify-debouncer-full 0.7.0, tokio 1.52.2, rusqlite 0.39.0, sysinfo 0.39.0, tauri 2.11.1, etc.)
- Context7 `/notify-rs/notify` — confirmed `RecommendedWatcher` + `new_debouncer` API patterns (Windows = `ReadDirectoryChangesW`)
- Context7 `/websites/v2_tauri_app` — confirmed Tauri v2 `Builder::default().setup()` pattern for spawning background tasks pre-window
- `.planning/research/STACK.md` — original stack research (used as input; corrected versions where current registry contradicts)
- `.planning/research/ARCHITECTURE.md` — full system layout (Phase 1 implements the Watcher Core + Goldberg adapter + Persistent Store layers; defers Schema Resolver, Notification Queue, Popup, Companion, Game Session)
- `.planning/research/PITFALLS.md` — pitfalls 1, 2, 3, 6, 7, 15 directly drive Phase 1 implementation choices
- `.planning/research/SUMMARY.md` — phase-1 implications confirmed
- `.planning/REQUIREMENTS.md` + `.planning/ROADMAP.md` — DETECT-01/05/06/07/08 requirement IDs and success criteria

### Secondary (MEDIUM confidence)
- WebSearch: Goldberg state file schema (`{ach_api_name: {earned, earned_time}}`) — confirmed by xan105/Achievement-Watcher wiki and 50t0r25/achievement-watchdog
- WebSearch: gbe_fork uses `%APPDATA%\GSE Saves\` instead of `%APPDATA%\Goldberg SteamEmu Saves\` — confirmed via Detanup01/gbe_fork README
- WebSearch: Steam libraryfolders.vdf in both `config\` (master, post-2022) and `steamapps\` (replicated) — confirmed from steamcommunity discussion and SubnauticaNitrox issue #142
- WebSearch: HKLM\SOFTWARE\Valve\Steam (32-bit) and WOW6432Node\Valve\Steam (64-bit) registry locations for Steam install path — confirmed across multiple Steam community / valkyrie issue #1056 sources

### Tertiary (LOW confidence — flagged for plan-phase validation)
- gbe_fork state file field names being identical to legacy Goldberg — A4 in Assumptions Log; one WebSearch result consistent with legacy schema, no canonical fork doc
- The `%PUBLIC%\Documents\Goldberg SteamEmu Saves\` path being live in modern installs — included as a cheap-to-check fallback root but not actively verified

## Project Constraints (from CLAUDE.md)

- **GSD workflow:** No edits outside GSD commands. The phase plan must be executed via `/gsd-execute-phase`.
- **Stack lock:** Tauri 2.11.1 + Rust 1.85+ + (later) React 19 + Vite 6. Cannot substitute Electron, WPF, etc.
- **Distribution:** Free + open-source on GitHub. License selection is a Phase 4 task; in Phase 1, ensure the repository has a placeholder LICENSE file or note its absence as a Phase 4 todo. (Repo today has no LICENSE; create one or leave to Phase 4 — recommend MIT to match the Tauri/Rust ecosystem default.)
- **Goldberg stance:** Passive detection only. Read state files; do NOT install/configure/recommend Goldberg setup. **Phase 1's UI surface is logs only — there is no setup wizard yet.**
- **Customization lock:** No theme/sound/position knobs in v1. Phase 1 has no UI so this is automatically satisfied.
- **Hobby pace:** No fixed deadline. Prefer reliable, well-tested patterns. Don't ship clever code for a 5% improvement when the standard library / standard crate handles it.
- **Windows-only v1:** `cfg(target_os = "windows")` is acceptable; no need to abstract for Linux/macOS.
- **No telemetry / no cloud:** Phase 1 makes zero HTTP calls. (Phase 2 will, for the Steam Web API schema fetch.)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all 16 versions verified against crates.io API on 2026-05-08
- Architecture: HIGH — adapter trait + watcher core pattern matches ARCHITECTURE.md spec which itself was high-confidence; refinements here (the explicit `seed_baseline` method, the dedup TTL design) are minor extensions
- Pitfalls: HIGH — drawn from PITFALLS.md (high confidence) and additional empirical cross-checks (gbe_fork path, libraryfolders.vdf location)
- Goldberg state file schema: MEDIUM-HIGH — three independent confirmations of `earned`/`earned_time` field names; A4 flag remains for gbe_fork-specific verification
- VDF parsing approach: HIGH — `keyvalues-parser` is the standard Rust ecosystem choice
- SQLite usage pattern: HIGH — rusqlite bundled is the de-facto standard for desktop Rust apps

**Research date:** 2026-05-08
**Valid until:** 2026-06-07 (30 days; stack is stable, Tauri 2.x is mature, no signaled breaking changes in notify/rusqlite/tokio in the next month)
