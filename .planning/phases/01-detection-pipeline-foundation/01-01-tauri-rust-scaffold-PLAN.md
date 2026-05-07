---
phase: 01-detection-pipeline-foundation
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - .gitignore
  - src-tauri/Cargo.toml
  - src-tauri/build.rs
  - src-tauri/tauri.conf.json
  - src-tauri/src/main.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/error.rs
  - tests/fixtures/goldberg/480/achievements.json
  - tests/fixtures/goldberg/README.md
  - .planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md
autonomous: true
requirements: [DETECT-01]
must_haves:
  truths:
    - "`cargo check --workspace` succeeds with zero errors"
    - "`cargo run --bin hallmark` (or `cargo tauri dev`) starts the Tauri Rust process and exits cleanly when killed"
    - "`tracing::info!(\"Hallmark starting\")` (or equivalent) is emitted to stdout at startup"
    - "Goldberg state-file schema (field names `earned` and `earned_time`) is empirically confirmed against a real save AND documented in NOTES.md"
  artifacts:
    - path: "Cargo.toml"
      provides: "Workspace root manifest declaring `src-tauri` member"
      contains: '[workspace]'
    - path: "src-tauri/Cargo.toml"
      provides: "Phase 1 crate dependencies pinned to exact versions"
      contains: 'notify-debouncer-full = "0.7"'
    - path: "src-tauri/src/main.rs"
      provides: "Binary entry calling library run()"
    - path: "src-tauri/src/lib.rs"
      provides: "Tauri builder skeleton + tracing init + setup() spawn point"
      contains: 'tauri::Builder'
    - path: "src-tauri/src/error.rs"
      provides: "thiserror-based error enums (PathDiscoveryError, AdapterError, StoreError)"
      contains: 'thiserror::Error'
    - path: "tests/fixtures/goldberg/480/achievements.json"
      provides: "Hand-crafted Goldberg state fixture for tests; appid 480 = Spacewar (Steam SDK demo)"
      contains: '"earned"'
    - path: ".planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md"
      provides: "Resolution of Assumption A4 — confirms gbe_fork field names match legacy Goldberg before adapter code is locked"
      contains: 'earned'
  key_links:
    - from: "src-tauri/src/main.rs"
      to: "src-tauri/src/lib.rs"
      via: "fn main calls hallmark_lib::run()"
      pattern: 'hallmark_lib::run'
    - from: "Cargo.toml"
      to: "src-tauri/Cargo.toml"
      via: "workspace members declaration"
      pattern: 'members\s*=\s*\[\s*"src-tauri"'
---

<objective>
Bootstrap the Hallmark Cargo workspace and Tauri v2 backend skeleton, pin every Phase 1 crate dependency at the exact version verified in RESEARCH.md (correcting STACK.md's stale `notify-debouncer-full = "0.5"` to `0.7`), wire `tracing-subscriber` for stdout logging, and resolve Assumption A4 by empirically inspecting a real Goldberg/gbe_fork state file before any adapter code is written.

Purpose: Every downstream plan depends on a buildable workspace, fixed dependency versions, working tracing, and a confirmed Goldberg state-file schema. Skipping the empirical schema inspection here would risk Plan 04's adapter parsing the wrong field names and silently emitting zero events.

Output:
- A buildable `cargo check --workspace` workspace
- `src-tauri/` Tauri v2 skeleton with all 16 Phase 1 crates pinned (no React frontend yet — `tauri.conf.json` declares no `windows` array)
- A canonical Goldberg state-file fixture under `tests/fixtures/goldberg/480/achievements.json` for downstream tests
- `empirical-goldberg-schema-NOTES.md` resolving A4 (either confirming `earned`/`earned_time` field names against a real save, or recording the divergence so Plan 04 can parameterize)
</objective>

<execution_context>
@C:/Users/reema/Documents/Programming/achievements/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/reema/Documents/Programming/achievements/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/STATE.md
@.planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md
@.planning/research/STACK.md
@.planning/research/ARCHITECTURE.md
@.planning/research/PITFALLS.md
@CLAUDE.md

<interfaces>
<!-- Versions verified against crates.io API on 2026-05-08 (see RESEARCH.md "Standard Stack"). -->
<!-- These are EXACT versions to pin — do NOT use `cargo add` without --version flags. -->

Crate versions for src-tauri/Cargo.toml:
```
tauri                  = "2.11"      # features: ["custom-protocol"] only when building release
tauri-build            = "2.11"      # build-dependency
tokio                  = "1.52"      # features: ["rt-multi-thread", "sync", "macros", "time", "fs"]
notify                 = "8.2"
notify-debouncer-full  = "0.7"       # CORRECTION vs STACK.md (0.5)
serde                  = "1.0"       # features: ["derive"]
serde_json             = "1.0"
rusqlite               = "0.39"      # features: ["bundled"]
keyvalues-parser       = "0.2"
winreg                 = "0.56"
walkdir                = "2.5"
sha2                   = "0.11"
anyhow                 = "1.0"
thiserror              = "2.0"
async-trait            = "0.1"
tracing                = "0.1"
tracing-subscriber     = "0.3"       # features: ["env-filter", "fmt"]
dirs                   = "6.0"
uuid                   = "1.23"      # features: ["v4", "serde"]
```

Module skeleton — files this plan creates as empty/stub modules so downstream plans can fill them:
```
src-tauri/src/
├── main.rs       (this plan: full implementation)
├── lib.rs        (this plan: full implementation)
├── error.rs      (this plan: full implementation)
├── paths.rs      (this plan: empty module stub)
├── sources/
│   └── mod.rs    (this plan: empty module stub)
├── watcher/
│   └── mod.rs    (this plan: empty module stub)
└── store/
    └── mod.rs    (this plan: empty module stub)
```

Goldberg state file shape (per RESEARCH.md "Goldberg state file parse + diff"):
```json
{
  "ACH_API_NAME_1": { "earned": true,  "earned_time": 1700000000 },
  "ACH_API_NAME_2": { "earned": false, "earned_time": 0          }
}
```
Top-level is an OBJECT (map), NOT an array. Field name is `earned` (boolean), NOT `unlocked`. Field name is `earned_time` (u64 unix seconds), NOT `unlock_time`.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Empirically resolve Assumption A4 — confirm Goldberg/gbe_fork state file field names</name>
  <files>
    - .planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md
    - tests/fixtures/goldberg/480/achievements.json
    - tests/fixtures/goldberg/README.md
  </files>
  <read_first>
    - .planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md (sections: "Goldberg state file parse + diff", "Assumptions Log A4", "Open Questions #1")
    - .planning/research/PITFALLS.md (Pitfall #15 — `unlock_time = 0`)
  </read_first>
  <action>
    RESEARCH.md flags Assumption A4 as LOW confidence: gbe_fork (the modern Goldberg fork that writes to `%APPDATA%\GSE Saves\`) MAY have diverged from legacy Goldberg's field names. Plan 04 will lock in the parser shape, so we resolve A4 NOW.

    Step 1 — Look for an existing real save on this machine. Run (in PowerShell, captured to NOTES.md):
    ```powershell
    Get-ChildItem -Path "$env:APPDATA\Goldberg SteamEmu Saves" -Recurse -Filter "achievements.json" -ErrorAction SilentlyContinue | Select-Object -First 3 FullName
    Get-ChildItem -Path "$env:APPDATA\GSE Saves" -Recurse -Filter "achievements.json" -ErrorAction SilentlyContinue | Select-Object -First 3 FullName
    ```
    For each found file, dump its contents (first 2KB) into the NOTES.md as a fenced JSON block, with the file path above it.

    Step 2 — Document the empirical findings in `.planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md` with this exact structure:
    ```markdown
    # Empirical Goldberg State File Schema (Assumption A4 Resolution)

    **Date:** {today}
    **Resolves:** RESEARCH.md Assumption A4, Open Question #1

    ## Method
    {what you ran, what you found / didn't find}

    ## Real Saves Inspected
    {for each: full path + JSON snippet, or "none found on this machine"}

    ## Field Names Confirmed
    | Field | Type | Found in legacy Goldberg | Found in gbe_fork | Notes |
    |-------|------|--------------------------|-------------------|-------|
    | earned | bool | {YES/NO/UNKNOWN} | {YES/NO/UNKNOWN} | |
    | earned_time | u64 | {YES/NO/UNKNOWN} | {YES/NO/UNKNOWN} | 0 indicates "earned but timestamp unknown" |

    ## Top-level Shape
    {OBJECT (map of api_name -> entry) | ARRAY of objects | OTHER}

    ## Decision for Plan 04
    {Lock parser to `{ "ACH_NAME": { "earned": bool, "earned_time": u64 } }` — OR — parameterize per-variant if divergence found}

    ## Conservative Fallback
    If no real save was available on this machine: parser uses the schema documented in RESEARCH.md (legacy Goldberg, three independent confirmations: xan105/Achievement-Watcher, achievement-watchdog, Goldberg readme). Plan 04 must include a `serde(default)` on `earned_time` AND tolerant `serde_json::Value` fallback so an unexpected shape from a future fork degrades to "skip silently with warning" not "panic".
    ```

    Step 3 — Author the canonical fixture. Steam appid 480 is Spacewar (the official Steamworks SDK demo) and is convention-safe to use as a test appid. Create `tests/fixtures/goldberg/480/achievements.json` with EXACTLY:
    ```json
    {
      "ACH_WIN_ONE_GAME": { "earned": true,  "earned_time": 1700000001 },
      "ACH_WIN_100_GAMES": { "earned": false, "earned_time": 0 },
      "ACH_TRAVEL_FAR_ACCUM": { "earned": false, "earned_time": 0 },
      "ACH_UNKNOWN_TIMESTAMP": { "earned": true, "earned_time": 0 }
    }
    ```
    The four entries cover: (a) earned with real timestamp, (b) unearned, (c) another unearned for diff testing, (d) earned-but-`earned_time=0` (PITFALLS.md #15 — must NOT use timestamp as unlock signal).

    Step 4 — Author `tests/fixtures/goldberg/README.md` with one paragraph explaining the fixture: appid 480 is Steamworks SDK Spacewar (a convention-safe non-real-game appid), the file is the Goldberg STATE file (not the SCHEMA file in `<game-dir>\steam_settings\`), and downstream tests treat it as an immutable reference.

    Step 5 — At the end of NOTES.md, in a "## Decision for Plan 04" section, write either: (a) "Schema confirmed — Plan 04 locks parser to `{api_name: {earned: bool, earned_time: u64}}`" if a real save was found and matches; or (b) "No real save available; relying on three independent secondary sources (RESEARCH.md). Plan 04 must add tolerant fallback (serde(default) on earned_time + serde_json::Value escape hatch)." Either decision is acceptable — Plan 04 reads this file and acts accordingly.

    Do NOT skip this task because "we already know the schema" — RESEARCH.md explicitly flagged it as A4 LOW confidence. The 5 minutes spent here saves Plan 04 from a silent zero-event bug.
  </action>
  <verify>
    <automated>powershell -NoProfile -Command "$ok = (Test-Path .planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md) -and (Test-Path tests/fixtures/goldberg/480/achievements.json) -and (Test-Path tests/fixtures/goldberg/README.md); if (-not $ok) { exit 1 }; $j = Get-Content tests/fixtures/goldberg/480/achievements.json -Raw | ConvertFrom-Json; if ($j.ACH_WIN_ONE_GAME.earned -ne $true) { exit 2 }; if ($j.ACH_UNKNOWN_TIMESTAMP.earned_time -ne 0) { exit 3 }; if ($j.ACH_UNKNOWN_TIMESTAMP.earned -ne $true) { exit 4 }; $n = Get-Content .planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md -Raw; if ($n -notmatch 'Decision for Plan 04') { exit 5 }; if ($n -notmatch 'earned' -or $n -notmatch 'earned_time') { exit 6 }; Write-Host 'A4 resolution complete'</automated>
  </verify>
  <acceptance_criteria>
    - File `.planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md` exists.
    - NOTES.md contains both the strings `earned` and `earned_time` (case-sensitive).
    - NOTES.md contains the heading `## Decision for Plan 04`.
    - NOTES.md contains the heading `## Real Saves Inspected` (even if the body is "none found on this machine").
    - File `tests/fixtures/goldberg/480/achievements.json` exists and parses as JSON via `ConvertFrom-Json`.
    - Fixture top-level is an OBJECT with at least 4 keys: `ACH_WIN_ONE_GAME`, `ACH_WIN_100_GAMES`, `ACH_TRAVEL_FAR_ACCUM`, `ACH_UNKNOWN_TIMESTAMP`.
    - `ACH_WIN_ONE_GAME.earned` is boolean true; `ACH_WIN_ONE_GAME.earned_time` is integer 1700000001.
    - `ACH_UNKNOWN_TIMESTAMP.earned` is boolean true AND `ACH_UNKNOWN_TIMESTAMP.earned_time` is integer 0 (covers PITFALLS.md #15).
    - File `tests/fixtures/goldberg/README.md` exists and explicitly states appid 480 is Steamworks SDK Spacewar.
  </acceptance_criteria>
  <done>A4 is resolved (confirmed or fallback documented), the fixture is committed, and Plan 04 has a deterministic schema reference plus a real-data sample to test against.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 2: Scaffold Cargo workspace + Tauri v2 src-tauri crate with all Phase 1 dependencies pinned</name>
  <files>
    - Cargo.toml
    - .gitignore
    - src-tauri/Cargo.toml
    - src-tauri/build.rs
    - src-tauri/tauri.conf.json
  </files>
  <read_first>
    - .planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md (sections: "Standard Stack", "Recommended Project Structure", "Don't Hand-Roll")
    - .planning/research/STACK.md (note: STACK.md says notify-debouncer-full = "0.5"; RESEARCH.md corrects this to "0.7" — use 0.7)
    - CLAUDE.md (Recommended Stack table — same `Tauri 2.11.1`, but use minor-version pin `2.11` in Cargo.toml)
  </read_first>
  <action>
    Create the workspace root + src-tauri crate manually (do NOT run `cargo create-tauri-app` — its scaffolding pulls in a React frontend, which Phase 1 explicitly does NOT have per RESEARCH.md "Architectural Responsibility Map / Frontend deferred to Phase 2").

    Step 1 — Workspace root `Cargo.toml` (verbatim):
    ```toml
    [workspace]
    members = ["src-tauri"]
    resolver = "2"

    [workspace.package]
    version    = "0.1.0"
    edition    = "2021"
    rust-version = "1.85"
    license    = "MIT"
    repository = "https://github.com/reemamazon44/hallmark"

    [profile.release]
    opt-level     = 3
    lto           = "thin"
    codegen-units = 1
    strip         = true
    panic         = "abort"
    ```

    Step 2 — `.gitignore` at workspace root (verbatim):
    ```
    /target
    /src-tauri/target
    /src-tauri/gen
    Cargo.lock.bak
    *.swp
    .DS_Store
    Thumbs.db
    /dist
    /node_modules
    /hallmark.db
    /hallmark.db-shm
    /hallmark.db-wal
    ```
    NOTE: keep `Cargo.lock` tracked (this is a binary crate, not a library).

    Step 3 — `src-tauri/Cargo.toml` (verbatim — pinning every Phase 1 dep at the version from RESEARCH.md "Standard Stack". Use minor-version pins to allow patch updates only.):
    ```toml
    [package]
    name         = "hallmark"
    version      = { workspace = true }
    edition      = { workspace = true }
    rust-version = { workspace = true }
    license      = { workspace = true }
    repository   = { workspace = true }
    description  = "PSN/Xbox-grade achievement satisfaction for PC gaming."
    publish      = false

    [build-dependencies]
    tauri-build = { version = "2.11", features = [] }

    [dependencies]
    tauri                 = { version = "2.11", features = [] }
    tokio                 = { version = "1.52", features = ["rt-multi-thread", "sync", "macros", "time", "fs"] }
    notify                = "8.2"
    notify-debouncer-full = "0.7"
    serde                 = { version = "1.0", features = ["derive"] }
    serde_json            = "1.0"
    rusqlite              = { version = "0.39", features = ["bundled"] }
    keyvalues-parser      = "0.2"
    walkdir               = "2.5"
    sha2                  = "0.11"
    anyhow                = "1.0"
    thiserror             = "2.0"
    async-trait           = "0.1"
    tracing               = "0.1"
    tracing-subscriber    = { version = "0.3", features = ["env-filter", "fmt"] }
    dirs                  = "6.0"
    uuid                  = { version = "1.23", features = ["v4", "serde"] }

    [target.'cfg(target_os = "windows")'.dependencies]
    winreg = "0.56"

    [features]
    default        = []
    custom-protocol = ["tauri/custom-protocol"]

    [[bin]]
    name = "hallmark"
    path = "src/main.rs"

    # NOTE: An additional `[[bin]] name = "hallmark-cli"` entry is added by Plan 05 once
    # bin/hallmark-cli.rs exists. Do NOT add it now — cargo errors on missing bin paths.

    [lib]
    name       = "hallmark_lib"
    path       = "src/lib.rs"
    crate-type = ["staticlib", "cdylib", "rlib"]
    ```

    Step 4 — `src-tauri/build.rs` (verbatim — required by tauri-build):
    ```rust
    fn main() {
        tauri_build::build();
    }
    ```

    Step 5 — `src-tauri/tauri.conf.json` (verbatim — minimal Tauri v2 config with NO windows array, NO frontend; the `app.frontendDist` field is intentionally a placeholder dist directory because Phase 1 has no built frontend):
    ```json
    {
      "$schema": "https://schema.tauri.app/config/2",
      "productName": "Hallmark",
      "version": "0.1.0",
      "identifier": "com.hallmark.app",
      "build": {
        "beforeDevCommand": "",
        "beforeBuildCommand": "",
        "frontendDist": "../dist",
        "devUrl": null
      },
      "app": {
        "windows": [],
        "security": {
          "csp": null
        }
      },
      "bundle": {
        "active": false,
        "targets": "none",
        "category": "Utility",
        "shortDescription": "PSN/Xbox-grade achievement popups for PC gaming",
        "longDescription": "PSN/Xbox-grade achievement popups for PC gaming.",
        "icon": []
      }
    }
    ```
    Note: `app.windows: []` is deliberate — Phase 1 has no UI. Phase 2 will populate this. `bundle.active: false` prevents `cargo tauri build` from trying to package an installer in Phase 1 (the bundling pipeline is Phase 4).

    After all five files exist, run:
    ```powershell
    cargo fetch --manifest-path src-tauri/Cargo.toml
    ```
    to download all dependencies and validate the manifest. This is a non-destructive check; Task 3 does the actual `cargo check`.
  </action>
  <verify>
    <automated>powershell -NoProfile -Command "if (-not (Test-Path Cargo.toml)) { exit 1 }; if (-not (Test-Path src-tauri/Cargo.toml)) { exit 2 }; if (-not (Test-Path src-tauri/build.rs)) { exit 3 }; if (-not (Test-Path src-tauri/tauri.conf.json)) { exit 4 }; if (-not (Test-Path .gitignore)) { exit 5 }; $c = Get-Content src-tauri/Cargo.toml -Raw; if ($c -notmatch 'notify-debouncer-full = .0\.7.') { exit 10 }; if ($c -notmatch 'tauri = \{ version = .2\.11.') { exit 11 }; if ($c -notmatch 'rusqlite = \{ version = .0\.39., features = \[.bundled.\] \}') { exit 12 }; if ($c -match 'notify-debouncer-full = .0\.5.') { exit 13 }; $t = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json; if ($t.app.windows.Count -ne 0) { exit 20 }; if ($t.bundle.active -ne $false) { exit 21 }; cargo fetch --manifest-path src-tauri/Cargo.toml --quiet; if ($LASTEXITCODE -ne 0) { exit 30 }; Write-Host 'scaffold ok'</automated>
  </verify>
  <acceptance_criteria>
    - `Cargo.toml` exists at workspace root and contains the literal string `members = ["src-tauri"]`.
    - `src-tauri/Cargo.toml` exists and contains the EXACT line `notify-debouncer-full = "0.7"` (NOT `"0.5"`).
    - `src-tauri/Cargo.toml` contains `tauri = { version = "2.11"` (or equivalent table form for tauri 2.11.x).
    - `src-tauri/Cargo.toml` contains `rusqlite = { version = "0.39", features = ["bundled"] }`.
    - `src-tauri/Cargo.toml` declares both `[[bin]] name = "hallmark"` AND `[lib] name = "hallmark_lib"`.
    - `src-tauri/Cargo.toml` does NOT yet declare a `hallmark-cli` bin target (Plan 05 adds it).
    - `src-tauri/tauri.conf.json` parses as JSON; `app.windows` is an empty array `[]`; `bundle.active` is boolean `false`.
    - `src-tauri/build.rs` exists and calls `tauri_build::build()`.
    - `.gitignore` contains `/target` and `/src-tauri/target` lines.
    - `cargo fetch --manifest-path src-tauri/Cargo.toml` exits 0 (resolves all 18 dependencies).
  </acceptance_criteria>
  <done>The workspace is buildable in principle: Cargo.toml resolves, every Phase 1 dep is pinned at the verified version, and the scaffold matches RESEARCH.md "Recommended Project Structure" with all module directories ready for Plans 02–05 to fill.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 3: Implement Rust entry points (main.rs, lib.rs, error.rs) + module stubs and verify cargo check passes</name>
  <files>
    - src-tauri/src/main.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/error.rs
    - src-tauri/src/paths.rs
    - src-tauri/src/sources/mod.rs
    - src-tauri/src/watcher/mod.rs
    - src-tauri/src/store/mod.rs
  </files>
  <read_first>
    - .planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md (sections: "Recommended Project Structure", "Pattern 2 — notify-debouncer-full"; section "Project Constraints" — backend only)
    - src-tauri/Cargo.toml (just-created — confirm `[lib] name = "hallmark_lib"` and dependency list before importing)
  </read_first>
  <action>
    Create all 7 source files. Stubs for `paths.rs`, `sources/mod.rs`, `watcher/mod.rs`, `store/mod.rs` are intentional — Plans 02–05 fill them in. The stubs MUST compile and be `pub mod`-declared from `lib.rs` so the dependency graph between plans works.

    Step 1 — `src-tauri/src/main.rs` (Windows GUI subsystem suppressed for now via `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` later in Phase 2; Phase 1 keeps console subsystem so stdout works for the CLI test harness):
    ```rust
    // Hallmark — PSN/Xbox-grade achievement notifications for PC gaming.
    // Phase 1 deliverable: backend-only detection pipeline. No UI yet.

    fn main() {
        hallmark_lib::run();
    }
    ```

    Step 2 — `src-tauri/src/lib.rs` (full implementation — Tauri builder skeleton + tracing init + setup hook reserved for spawning background tasks in Plans 04 and 05):
    ```rust
    //! Hallmark library entry point.
    //!
    //! Phase 1 scope: tracing initialization, Tauri builder skeleton with empty `windows`
    //! array, and a `setup()` hook that LATER plans (04 watcher, 05 dedup+cli) attach
    //! background tokio tasks to. This file establishes the structure; downstream plans
    //! extend `setup()` rather than restructuring.

    pub mod error;
    pub mod paths;
    pub mod sources;
    pub mod store;
    pub mod watcher;

    use tracing_subscriber::EnvFilter;

    /// Initialize structured logging. Call once at process start.
    /// Reads RUST_LOG env var; defaults to `hallmark_lib=info,warn` for clean output.
    pub fn init_tracing() {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("hallmark_lib=info,warn"));
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_level(true)
            .try_init();
    }

    /// Production entry — invoked by `bin/main.rs`. Starts the Tauri shell.
    /// Phase 1: Tauri starts but creates NO windows (windows array empty in tauri.conf.json).
    /// The process stays alive via Tauri's run loop; Plans 04/05 spawn background tasks
    /// inside the `setup()` closure.
    pub fn run() {
        init_tracing();
        tracing::info!(
            version = env!("CARGO_PKG_VERSION"),
            "Hallmark starting (Phase 1 — backend only, no UI)"
        );

        tauri::Builder::default()
            .setup(|_app| {
                // Plans 04 + 05 attach pipeline tasks here:
                //   tokio::spawn(watcher::run_watcher(...));
                //   tokio::spawn(cli::run_cli_sink(...));
                tracing::info!("Tauri setup complete (no background tasks attached in Phase 1 scaffold)");
                Ok(())
            })
            .run(tauri::generate_context!())
            .expect("Tauri runtime failed to start");
    }
    ```

    Step 3 — `src-tauri/src/error.rs` (full implementation — every Phase 1 module imports its enum from here; Plan 02 expands StoreError, Plan 03 expands PathDiscoveryError, Plan 04 expands AdapterError):
    ```rust
    //! Domain error types. Each module owns a thiserror enum below.
    //! Application code uses `anyhow::Result`; library boundaries return these typed errors.

    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum PathDiscoveryError {
        #[error("registry read failed: {0}")]
        Registry(#[from] std::io::Error),
        #[error("VDF parse failed at {path}: {message}")]
        Vdf { path: std::path::PathBuf, message: String },
        #[error("path does not exist: {0}")]
        NotFound(std::path::PathBuf),
    }

    #[derive(Debug, Error)]
    pub enum AdapterError {
        #[error("io: {0}")]
        Io(#[from] std::io::Error),
        #[error("json parse: {0}")]
        Json(#[from] serde_json::Error),
        #[error("invalid path layout: expected <root>/<appid>/achievements.json, got {0}")]
        InvalidLayout(std::path::PathBuf),
    }

    #[derive(Debug, Error)]
    pub enum StoreError {
        #[error("sqlite: {0}")]
        Sqlite(#[from] rusqlite::Error),
        #[error("system time: {0}")]
        SystemTime(#[from] std::time::SystemTimeError),
    }
    ```

    Step 4 — `src-tauri/src/paths.rs` (stub — Plan 03 fills):
    ```rust
    //! Path discovery: Steam install registry, libraryfolders.vdf, Goldberg local_save.txt.
    //! Implemented by Plan 03 (DETECT-08).
    ```

    Step 5 — `src-tauri/src/sources/mod.rs` (stub — Plan 02 fills with SourceAdapter trait + RawUnlockEvent + SourceKind):
    ```rust
    //! Source adapter trait and event types.
    //! Implemented by Plan 02. Plan 04 adds `pub mod goldberg;`.
    ```

    Step 6 — `src-tauri/src/watcher/mod.rs` (stub — Plan 04 fills WatcherCore, Plan 05 adds `pub mod dedup;`):
    ```rust
    //! Watcher core: notify-debouncer-full driver. Implemented by Plan 04 (DETECT-06).
    //! Plan 05 adds `pub mod dedup;` for cross-source dedup (DETECT-07).
    ```

    Step 7 — `src-tauri/src/store/mod.rs` (stub — Plan 02 fills SqliteStore + migrations + queries):
    ```rust
    //! SQLite-backed persistence. Implemented by Plan 02.
    ```

    Step 8 — Run the full check:
    ```powershell
    cargo check --manifest-path src-tauri/Cargo.toml --all-targets
    ```
    Must succeed with zero errors. Warnings about unused imports in `error.rs` are EXPECTED (the enums are wired up by later plans) — leave them; do NOT add `#[allow(dead_code)]` because Plan 02 will use the types and `cargo check --all-targets` post-Plan-02 should be warning-clean.
  </action>
  <verify>
    <automated>powershell -NoProfile -Command "if (-not (Test-Path src-tauri/src/main.rs)) { exit 1 }; if (-not (Test-Path src-tauri/src/lib.rs)) { exit 2 }; if (-not (Test-Path src-tauri/src/error.rs)) { exit 3 }; if (-not (Test-Path src-tauri/src/paths.rs)) { exit 4 }; if (-not (Test-Path src-tauri/src/sources/mod.rs)) { exit 5 }; if (-not (Test-Path src-tauri/src/watcher/mod.rs)) { exit 6 }; if (-not (Test-Path src-tauri/src/store/mod.rs)) { exit 7 }; $l = Get-Content src-tauri/src/lib.rs -Raw; if ($l -notmatch 'pub mod error;') { exit 10 }; if ($l -notmatch 'pub mod paths;') { exit 11 }; if ($l -notmatch 'pub mod sources;') { exit 12 }; if ($l -notmatch 'pub mod store;') { exit 13 }; if ($l -notmatch 'pub mod watcher;') { exit 14 }; if ($l -notmatch 'tracing_subscriber::fmt') { exit 15 }; if ($l -notmatch 'tauri::Builder::default') { exit 16 }; if ($l -notmatch 'init_tracing') { exit 17 }; $m = Get-Content src-tauri/src/main.rs -Raw; if ($m -notmatch 'hallmark_lib::run\(\)') { exit 20 }; $e = Get-Content src-tauri/src/error.rs -Raw; if ($e -notmatch 'PathDiscoveryError') { exit 30 }; if ($e -notmatch 'AdapterError') { exit 31 }; if ($e -notmatch 'StoreError') { exit 32 }; cargo check --manifest-path src-tauri/Cargo.toml --all-targets 2>&1 | Tee-Object -Variable out; if ($LASTEXITCODE -ne 0) { Write-Host $out; exit 40 }; Write-Host 'cargo check OK'</automated>
  </verify>
  <acceptance_criteria>
    - All 7 files exist in `src-tauri/src/`.
    - `src-tauri/src/lib.rs` declares all 5 modules: `pub mod error;`, `pub mod paths;`, `pub mod sources;`, `pub mod store;`, `pub mod watcher;`.
    - `src-tauri/src/lib.rs` defines `pub fn init_tracing()` AND `pub fn run()`.
    - `src-tauri/src/lib.rs` calls `tracing_subscriber::fmt()` (the tracing initialization).
    - `src-tauri/src/lib.rs` calls `tauri::Builder::default()` AND `tauri::generate_context!()`.
    - `src-tauri/src/main.rs` is exactly the 5-line idiomatic main calling `hallmark_lib::run()`.
    - `src-tauri/src/error.rs` defines all three enums: `PathDiscoveryError`, `AdapterError`, `StoreError` — each with `#[derive(Debug, Error)]`.
    - `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` exits 0.
    - The four stub files (`paths.rs`, `sources/mod.rs`, `watcher/mod.rs`, `store/mod.rs`) are present and contain at minimum a doc-comment referencing the plan that fills them (Plan 02 / 03 / 04 / 05).
  </acceptance_criteria>
  <done>The Hallmark workspace builds cleanly with `cargo check --all-targets`. The Tauri Rust skeleton is in place with tracing wired and a `setup()` hook ready for Plans 04/05 to extend. All five `src-tauri/src/` modules are declared (some as stubs) so downstream plans don't trigger module-not-found errors during their own cargo checks.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| disk → process | Goldberg state files (`%APPDATA%\Goldberg SteamEmu Saves\<appid>\achievements.json`) are user-writable JSON. Untrusted JSON crosses this boundary at every file event. (Phase 1 only sets up the scaffold; Plan 04 enforces.) |
| registry → process | `HKLM\...\Steam\InstallPath` and `HKCU\...\Steam\SteamPath` are read at startup. Plan 03 enforces; Phase 1 scaffold has no registry reads. |
| stdin / argv → process | Phase 1 scaffold has no CLI flags yet (Plan 05 adds `--override-goldberg-root`). Tauri ingests no untrusted input in this phase. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-01-S1 | Spoofing | Cargo dependency supply chain | accept | Workspace pins exact minor versions verified against crates.io API on 2026-05-08; `Cargo.lock` is committed (binary crate convention) so future builds are reproducible. No alternate registries. |
| T-01-T1 | Tampering | tauri.conf.json bundle.active flag | mitigate | `bundle.active = false` in scaffold prevents accidental release-grade bundling in Phase 1. Phase 4 flips this with intent. |
| T-01-I1 | Info disclosure | tracing logs to stdout | mitigate | Default filter `hallmark_lib=info,warn` — no debug-level secrets. RUST_LOG override is a developer-time facility; production builds never expose secrets because there are none in Phase 1 (no env vars, no API keys, no auth). |
| T-01-D1 | Denial of service | cargo build pulling unbounded deps | mitigate | Workspace `Cargo.toml` declares `members = ["src-tauri"]` only — no transitive workspace explosion. `[profile.release]` strips debug info; minor-version pins prevent SemVer-major drift. |
</threat_model>

<verification>
End-of-plan checks (run sequentially):
```powershell
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
Test-Path .planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md
Test-Path tests/fixtures/goldberg/480/achievements.json
Get-Content tests/fixtures/goldberg/480/achievements.json -Raw | ConvertFrom-Json | Format-List
```
The first two commands MUST exit 0. `cargo fmt --check` exit-1 indicates formatting drift; run `cargo fmt` to fix.
</verification>

<success_criteria>
- `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` returns exit code 0 with zero errors.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` returns exit code 0.
- `cargo fetch --manifest-path src-tauri/Cargo.toml` resolves all 18 dependencies (16 main + tauri-build + winreg-as-Windows-only).
- `notify-debouncer-full` is pinned at version `0.7` (the STACK.md `0.5` is corrected per RESEARCH.md verification).
- Empirical Goldberg schema decision (Assumption A4) is documented and committed to `.planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md`.
- Plans 02 and 03 can begin work in Wave 2 with all 5 module stubs (`error.rs`, `paths.rs`, `sources/mod.rs`, `store/mod.rs`, `watcher/mod.rs`) already declared from `lib.rs`.
</success_criteria>

<output>
After completion, create `.planning/phases/01-detection-pipeline-foundation/01-01-SUMMARY.md`
documenting: workspace scaffold completed; all 16 Phase 1 crates pinned (notify-debouncer-full
corrected to 0.7); A4 resolution status; module stub structure; what Plans 02/03/04/05 each
need to fill in.
</output>
