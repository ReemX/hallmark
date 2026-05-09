---
phase: 04-polish-distribution
plan: 01b
type: execute
wave: 2
depends_on:
  - "04-01a"
files_modified:
  - src-tauri/tauri.conf.json
  - src-tauri/src/lib.rs
autonomous: true
requirements:
  - POL-01
  - POL-02
  - DIST-01
  - DIST-02
  - DIST-04
tags:
  - tauri-config
  - integration-spine
  - phase4-foundation-b

must_haves:
  truths:
    - "tauri.conf.json `bundle.active = true`, `bundle.targets = [\"nsis\"]`, `bundle.createUpdaterArtifacts = true`, `bundle.windows.nsis.installMode = \"perUser\"`, and `plugins.updater.endpoints` populated"
    - "CSP `connect-src` whitelists `https://github.com` and `https://objects.githubusercontent.com` (updater fetches)"
    - "lib.rs::run() declares 7 new Phase 4 modules (`tray`, `autostart`, `test_trigger`, `first_run`, `settings_window`, `portable_mode`, `updater_glue`) via `pub mod` lines and the full crate compiles"
    - "AppState gains 5 new fields: `raw_tx`, `portable_mode`, `silent_launch`, `pending_update`, `cached_discovery` — all populated in setup()"
    - "Four new Tauri commands exist and are registered in `tauri::generate_handler!`: `rescan_paths`, `install_pending_update` (stub), `wizard_dismiss`, `open_settings_window`"
    - "Updater plugin is registered via `.plugin(tauri_plugin_updater::Builder::new().build())` in the Tauri builder chain"
    - "setup() calls `tray::build_tray`, `updater_glue::spawn_background_check`, `first_run::open_wizard`, `test_trigger::seed_test_fixture` — each in its prescribed location with surrounding warn-and-continue handling"
    - "`cargo build --lib && cargo build --bin hallmark` succeeds. Phase 1-3 pipeline still runs end-to-end on `cargo tauri dev`"
    - "Round-trip tests from 04-01a's queries.rs additions now pass (the module ladder is complete enough for `cargo test --lib`)"
  artifacts:
    - path: "src-tauri/tauri.conf.json"
      provides: "Bundle active + NSIS perUser + updater plugin config + CSP for GitHub"
      contains: '"installMode": "perUser"'
    - path: "src-tauri/src/lib.rs"
      provides: "Module declarations + AppState extension + setup() integration spine + 4 new commands"
      contains: 'pub mod tray'
  key_links:
    - from: "src-tauri/src/lib.rs"
      to: "src-tauri/src/tray.rs::build_tray"
      via: ".setup() closure — `crate::tray::build_tray(app)?;`"
      pattern: "tray::build_tray"
    - from: "src-tauri/src/lib.rs"
      to: "tauri_plugin_updater::Builder"
      via: ".plugin() chain"
      pattern: "tauri_plugin_updater::Builder"
    - from: "src-tauri/src/lib.rs::AppState"
      to: "raw_tx clone for D-04 test injection"
      via: "AppState struct field"
      pattern: "pub raw_tx: tokio::sync::mpsc::Sender"
    - from: "src-tauri/tauri.conf.json"
      to: "GitHub Releases latest.json"
      via: "plugins.updater.endpoints"
      pattern: "releases/latest/download/latest.json"
---

<objective>
Foundation B of the Phase 4 integration spine. This plan extends
`tauri.conf.json` (bundle + updater config + CSP) and `lib.rs` (module
declarations + AppState extension + 4 new commands + setup() wiring). After
this plan completes, the integration spine is fully formed: every Phase 4
entry point is called from setup(), each module's body is a stub that logs
WARN, and Wave 2 plans (04-02, 04-03, 04-04, 04-05) can swap stubs for real
implementations without touching `lib.rs`, `Cargo.toml`, `tauri.conf.json`,
`vite.config.ts`, or capability JSONs.

Purpose: Concentrate the high-coupling spine work (lib.rs + tauri.conf.json)
into a single plan so Wave 2 plans don't have to re-merge into these files.
This was the second half of the original 04-01; splitting from 04-01a halves
the per-plan context cost.

Output: Configuration is publication-ready except for the real updater pubkey
(Plan 04-06 injects after Ed25519 keypair generation). The Phase 1-3 popup +
companion windows continue to work because `bundle.active` only affects
build-time bundling, not runtime.
</objective>

<execution_context>
@C:/Users/reema/Documents/Programming/achievements/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/reema/Documents/Programming/achievements/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md
@.planning/phases/04-polish-distribution/04-CONTEXT.md
@.planning/phases/04-polish-distribution/04-RESEARCH.md
@.planning/phases/04-polish-distribution/04-PATTERNS.md
@.planning/phases/04-polish-distribution/04-UI-SPEC.md
@.planning/phases/04-polish-distribution/04-01a-PLAN.md
@CLAUDE.md

@src-tauri/tauri.conf.json
@src-tauri/src/lib.rs
@src-tauri/src/sources/mod.rs
@src-tauri/src/paths.rs
@src-tauri/src/store/queries.rs
@src-tauri/src/tray.rs
@src-tauri/src/autostart.rs
@src-tauri/src/test_trigger.rs
@src-tauri/src/portable_mode.rs
@src-tauri/src/first_run.rs
@src-tauri/src/settings_window.rs
@src-tauri/src/updater_glue.rs

<interfaces>
<!-- Module stubs from 04-01a — DO NOT modify, only call from setup(): -->

```rust
// src-tauri/src/tray.rs
pub fn build_tray(app: &App) -> tauri::Result<()>;

// src-tauri/src/autostart.rs (cross-platform stubs)
pub fn is_enabled() -> anyhow::Result<bool>;
pub fn enable() -> anyhow::Result<()>;
pub fn disable() -> anyhow::Result<()>;

// src-tauri/src/test_trigger.rs
pub const TEST_API_NAME: &str;
pub const TEST_APP_ID: u64;
pub fn fire(app: &AppHandle) -> anyhow::Result<()>;
pub fn seed_test_fixture(store: &crate::store::SqliteStore) -> anyhow::Result<()>;

// src-tauri/src/portable_mode.rs
pub fn is_portable() -> bool;        // stub returns false
pub fn is_silent_launch() -> bool;   // real impl

// src-tauri/src/first_run.rs
pub fn open_wizard(app: AppHandle, any_path_detected: bool) -> tauri::Result<()>;

// src-tauri/src/settings_window.rs
pub fn open(app: &AppHandle) -> tauri::Result<()>;

// src-tauri/src/updater_glue.rs
pub fn spawn_background_check(app: AppHandle);
```

<!-- queries.rs additions from 04-01a (already merged): -->

```rust
pub fn get_first_run_done(conn: &Connection) -> anyhow::Result<bool>;
pub fn set_first_run_done(conn: &Connection) -> anyhow::Result<()>;
pub fn get_last_update_check(conn: &Connection) -> anyhow::Result<Option<i64>>;
pub fn set_last_update_check(conn: &Connection, unix_secs: i64) -> anyhow::Result<()>;
```

<!-- Existing AppState (PRE-extension): -->

```rust
pub struct AppState {
    pub store: Arc<crate::store::SqliteStore>,
    pub schema: crate::schema::SchemaCache,
    pub session_id: String,
}
```

</interfaces>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: tauri.conf.json bundle + updater config + CSP whitelist</name>
  <files>src-tauri/tauri.conf.json</files>
  <read_first>
    src-tauri/tauri.conf.json,
    .planning/phases/04-polish-distribution/04-RESEARCH.md (sections "Pattern 4 tauri-plugin-updater wiring" lines 578-600, "Pattern 5 Static latest.json", "Architecture Patterns", "Common Pitfalls #6 latest.json URL"),
    .planning/phases/04-polish-distribution/04-PATTERNS.md (section "src-tauri/tauri.conf.json (extended)" lines 510-543),
    .planning/phases/04-polish-distribution/04-CONTEXT.md (D-21, D-22, D-25)
  </read_first>
  <action>
    Replace the existing `bundle` block in `src-tauri/tauri.conf.json` with:
    ```json
    "bundle": {
      "active": true,
      "createUpdaterArtifacts": true,
      "targets": ["nsis"],
      "category": "Utility",
      "shortDescription": "PSN/Xbox-grade achievement popups for PC gaming",
      "longDescription": "PSN/Xbox-grade achievement popups for PC gaming.",
      "icon": ["icons/icon.ico"],
      "windows": {
        "nsis": {
          "installMode": "perUser"
        }
      }
    }
    ```
    `active: true` flips Phase 1-3 default. `targets: ["nsis"]` is explicit per RESEARCH Pitfall 7 / Architecture Patterns — Tauri does NOT have a portable .zip target; portable is a custom CI step in Plan 04-06. `installMode: "perUser"` (D-22) keeps installs in `%LOCALAPPDATA%\Hallmark` with no UAC prompt.

    Add a top-level `plugins` object after `bundle` (or merge if one exists):
    ```json
    "plugins": {
      "updater": {
        "pubkey": "PLACEHOLDER_REPLACE_AT_RELEASE",
        "endpoints": [
          "https://github.com/ReemX/hallmark/releases/latest/download/latest.json"
        ]
      }
    }
    ```
    The `pubkey` value is a literal placeholder string `PLACEHOLDER_REPLACE_AT_RELEASE` — Plan 04-06 generates the real Ed25519 keypair via `tauri signer generate` and replaces this string with the real public key. Until then, `tauri build` will warn but still compile (the updater check runtime-fails gracefully). The endpoint URL uses `ReemX/hallmark` per the git config in this repo (verify the repo URL via `git remote get-url origin`; if different, substitute the actual `<owner>/<repo>` slug).

    Update CSP `connect-src` in `app.security.csp`. Current value:
    ```
    connect-src 'self' https://api.steampowered.com
    ```
    New value (append GitHub release-asset CDN hosts):
    ```
    connect-src 'self' https://api.steampowered.com https://github.com https://objects.githubusercontent.com
    ```
    Both new hosts are required: `github.com` for the redirect at `/releases/latest/download/`, `objects.githubusercontent.com` for the actual asset blob. This change UNBLOCKS `tauri-plugin-updater` HTTP fetches; without it, updater check fails with CSP violation.

    Do NOT modify `app.windows`, `productName`, `version`, `identifier`, `build`, or any other field outside `bundle`, `plugins`, and the `connect-src` clause inside `app.security.csp`. JSON does not support comments; Plan 04-06 will inject the real pubkey.
  </action>
  <verify>
    <automated>
      cd C:/Users/reema/Documents/Programming/achievements &amp;&amp;
      python -c "import json; c = json.load(open('src-tauri/tauri.conf.json')); assert c['bundle']['active'] == True; assert c['bundle']['targets'] == ['nsis']; assert c['bundle']['createUpdaterArtifacts'] == True; assert c['bundle']['windows']['nsis']['installMode'] == 'perUser'; assert 'updater' in c['plugins']; assert 'releases/latest/download/latest.json' in c['plugins']['updater']['endpoints'][0]; assert 'https://github.com' in c['app']['security']['csp']; assert 'https://objects.githubusercontent.com' in c['app']['security']['csp']; print('OK')"
    </automated>
  </verify>
  <acceptance_criteria>
    - `tauri.conf.json` validates as JSON.
    - `bundle.active == true`, `bundle.targets == ["nsis"]`, `bundle.createUpdaterArtifacts == true`, `bundle.windows.nsis.installMode == "perUser"`.
    - `plugins.updater.endpoints[0]` contains `releases/latest/download/latest.json`.
    - `plugins.updater.pubkey` is a non-empty string (placeholder OK).
    - `app.security.csp` `connect-src` clause contains both `https://github.com` and `https://objects.githubusercontent.com`.
    - All other Phase 1-3 fields (productName, identifier, build, app.windows, etc.) are unchanged byte-for-byte except where listed above.
  </acceptance_criteria>
  <done>
    Configuration is publication-ready except for the real pubkey, which Plan 04-06 injects after generating the Ed25519 keypair.
  </done>
</task>

<task type="auto" tdd="false">
  <name>Task 2: lib.rs setup() integration spine — module declarations + AppState extension + plugin reg + 4 commands + setup() ordering</name>
  <files>src-tauri/src/lib.rs</files>
  <read_first>
    src-tauri/src/lib.rs (full — focus on lines 8-21 module ladder, 32-43 AppState, 156-356 setup() body),
    src-tauri/src/sources/mod.rs (lines 36-49 RawUnlockEvent),
    src-tauri/src/tray.rs (the 04-01a stub — confirm signature),
    src-tauri/src/autostart.rs (the 04-01a stub),
    src-tauri/src/test_trigger.rs (the 04-01a stub),
    src-tauri/src/portable_mode.rs (the 04-01a stub),
    src-tauri/src/first_run.rs (the 04-01a stub),
    src-tauri/src/settings_window.rs (the 04-01a stub),
    src-tauri/src/updater_glue.rs (the 04-01a stub),
    src-tauri/src/store/queries.rs (the 04-01a additions),
    .planning/phases/04-polish-distribution/04-RESEARCH.md (Pattern 4 lines 537-621 updater wiring, "Architecture Patterns" diagram lines 230-263, Architecture Patterns "Pattern 6" first-run trigger lines 678-693),
    .planning/phases/04-polish-distribution/04-PATTERNS.md (section "src-tauri/src/lib.rs (extended setup())" lines 391-483)
  </read_first>
  <action>
    Modify `src-tauri/src/lib.rs` in three regions. PRESERVE all Phase 1-3 code unchanged — only ADD.

    **Region 1: module ladder.** After the existing Phase 2 module declarations (line 21 `pub mod game_detect;`), append:
    ```rust
    // ---- Phase 4 modules ----
    // 04-01a created the file stubs; this plan declares them in the ladder
    // and 04-02/03/04/05 fill in each module's body.
    pub mod tray;
    pub mod autostart;
    pub mod test_trigger;
    pub mod first_run;
    pub mod settings_window;
    pub mod portable_mode;
    pub mod updater_glue;
    ```

    **Region 2: AppState extension.** Inside `pub mod commands { ... }`, modify the `AppState` struct to add 5 new fields. The existing 3 fields (store, schema, session_id) stay first — append the new ones with comments:
    ```rust
    pub struct AppState {
        pub store: Arc<crate::store::SqliteStore>,
        pub schema: crate::schema::SchemaCache,
        pub session_id: String,
        // ---- Phase 4 additions ----
        /// Clone of the adapter→dedup mpsc::Sender for D-04 test-popup injection.
        pub raw_tx: tokio::sync::mpsc::Sender<crate::sources::RawUnlockEvent>,
        /// True if running outside `%LOCALAPPDATA%\Hallmark` (D-23 — disables updater).
        pub portable_mode: bool,
        /// True if launched with `--silent` (D-08 — companion does NOT auto-show).
        pub silent_launch: bool,
        /// Stash for tauri_plugin_updater::Update awaiting modal confirmation (D-18).
        pub pending_update: Arc<tokio::sync::Mutex<Option<tauri_plugin_updater::Update>>>,
        /// Cached DiscoveredPaths from startup — Settings/Wizard rescan replaces this.
        pub cached_discovery: Arc<tokio::sync::RwLock<crate::paths::DiscoveredPaths>>,
    }
    ```
    Note: `pending_update` uses `tokio::sync::Mutex` (not std::sync::Mutex) because it's awaited across `await` points in the updater install command. `cached_discovery` uses `RwLock` because Settings/Wizard rescan is rare (write-locked) but multiple readers (Tauri command handlers) may snapshot it.

    Add four new Tauri command stubs to the `pub mod commands { ... }` module — these are the invoke handlers Wave 2 plans wire to React pages:

    ```rust
    /// D-15/D-16/D-17: Settings → Detected sources → Rescan, and Wizard initial state.
    /// Plan 04-04 (settings) and 04-05 (wizard) finalize the body shape.
    #[tauri::command]
    pub async fn rescan_paths(
        state: tauri::State<'_, AppState>,
    ) -> Result<crate::paths::DiscoveredPaths, String> {
        let fresh = tokio::task::spawn_blocking(|| crate::paths::discover())
            .await
            .map_err(|e| e.to_string())?;
        let mut guard = state.cached_discovery.write().await;
        *guard = fresh.clone();
        Ok(fresh)
    }

    /// D-20: Modal "Install" button. Calls update.download_and_install + app.restart().
    /// Plan 04-04 finalizes the implementation; this stub returns a clear error
    /// so the React modal surfaces it instead of hanging.
    #[tauri::command]
    pub async fn install_pending_update(
        app: tauri::AppHandle,
        state: tauri::State<'_, AppState>,
    ) -> Result<(), String> {
        let _ = (&app, &state);
        Err("install_pending_update STUB — Plan 04-04 not yet implemented".into())
    }

    /// D-14: Wizard "Get started" / "Continue anyway" — sets first_run_done if any path detected.
    /// Plan 04-05 finalizes; this stub does the SQLite write so dismissal works end-to-end now.
    #[tauri::command]
    pub async fn wizard_dismiss(
        app: tauri::AppHandle,
        state: tauri::State<'_, AppState>,
    ) -> Result<(), String> {
        let cached = state.cached_discovery.read().await;
        let any = !cached.steam_libraries.is_empty()
            || !cached.goldberg_save_roots.is_empty()
            || !cached.cream_api_appid_dirs.is_empty()
            || !cached.sse_appid_dirs.is_empty()
            || cached.steam_legit_appcache_stats.is_some();
        drop(cached);
        if any {
            state.store.with_conn(|c| crate::store::queries::set_first_run_done(c))
                .map_err(|e| e.to_string())?;
        }
        if let Some(w) = tauri::Manager::get_webview_window(&app, "wizard") {
            let _ = w.close();
        }
        Ok(())
    }

    /// D-01 tray "Settings…" item — opens (or focuses) the Settings window from a frontend invoke.
    /// Plan 04-04 owns the actual builder; this stub delegates to settings_window::open.
    #[tauri::command]
    pub fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
        crate::settings_window::open(&app).map_err(|e| e.to_string())
    }
    ```

    **Region 3: setup() body extensions.** Insert blocks at the precisely-numbered points listed below (preserve all existing comments/code; INSERT new blocks, never replace).

    1. After step 3 (path discovery, ~line 186), capture cached_discovery:
       ```rust
       let cached_discovery = std::sync::Arc::new(tokio::sync::RwLock::new(discovery.clone()));
       ```

    2. After step 5 (adapter list, ~line 220), AppState's `raw_tx` clone is needed before step 6 creates the channel — REORDER: declare cached_discovery and the placeholder for raw_tx_clone here. Actually, raw_tx is created at step 6 (`let (raw_tx, raw_rx) = ...`). Clone IMMEDIATELY after that line, BEFORE it gets moved into `tauri::async_runtime::spawn(watcher::run_watcher(adapters, raw_tx))` at step 12:
       ```rust
       // Phase 4: clone raw_tx for AppState (D-04 test-popup inject seam).
       let raw_tx_for_state = raw_tx.clone();
       ```
       Place this immediately after the existing `let (raw_tx, raw_rx) = tokio::sync::mpsc::channel::<sources::RawUnlockEvent>(64);` line.

    3. Detect portable_mode + silent_launch BEFORE step 10 (AppState management):
       ```rust
       // Phase 4: portable detection + --silent argv parsing.
       let portable_mode = portable_mode::is_portable();
       let silent_launch = portable_mode::is_silent_launch();
       tracing::info!(portable_mode, silent_launch, "Phase 4 startup flags");
       let pending_update: std::sync::Arc<tokio::sync::Mutex<Option<tauri_plugin_updater::Update>>>
           = std::sync::Arc::new(tokio::sync::Mutex::new(None));
       ```

    4. EXTEND step 10 — replace the existing `app.manage(AppState { ... })` block with the 8-field version:
       ```rust
       app.manage(AppState {
           store: store.clone(),
           schema: schema_cache.clone(),
           session_id: session_id.clone(),
           raw_tx: raw_tx_for_state,
           portable_mode,
           silent_launch,
           pending_update: pending_update.clone(),
           cached_discovery: cached_discovery.clone(),
       });
       ```

    5. After step 10 (AppState management), seed the test fixture into schema_cache:
       ```rust
       // Phase 4 D-05: pre-seed schema_cache for the synthetic test popup so the
       // Fire-test menu item produces a fully-resolved popup without Web API roundtrip.
       if let Err(e) = test_trigger::seed_test_fixture(&store) {
           tracing::warn!(error = %e, "test_trigger::seed_test_fixture failed; test popup may show fallback display name");
       }
       ```

    6. After step 14 (game-started listener, ~line 353), append the Phase 4 wiring block:
       ```rust
       // ----- Phase 4 wiring -----
       // Build tray icon + menu (Plan 04-02 owns body).
       if let Err(e) = tray::build_tray(app) {
           tracing::warn!(error = %e, "tray icon failed to build; continuing without tray");
       } else {
           tracing::info!("tray icon registered");
       }

       // Updater background-check (Plan 04-04 owns body). Skips when portable_mode.
       if !portable_mode {
           updater_glue::spawn_background_check(app_handle.clone());
       } else {
           tracing::info!("portable mode: updater background-check skipped (D-23)");
       }

       // First-run wizard logic (Plan 04-05 owns body). D-14: open if flag unset
       // OR if 0 paths detected on this launch.
       let first_run_done = store.with_conn(|c| crate::store::queries::get_first_run_done(c))?;
       let any_path_detected = !discovery.steam_libraries.is_empty()
           || !discovery.goldberg_save_roots.is_empty()
           || !discovery.cream_api_appid_dirs.is_empty()
           || !discovery.sse_appid_dirs.is_empty()
           || discovery.steam_legit_appcache_stats.is_some();
       if !first_run_done || !any_path_detected {
           if let Err(e) = first_run::open_wizard(app_handle.clone(), any_path_detected) {
               tracing::warn!(error = %e, "first-run wizard failed to open");
           } else {
               tracing::info!(any_path_detected, "first-run wizard opened");
           }
       } else {
           tracing::debug!("first_run_done set and ≥1 path detected — wizard skipped");
       }
       ```

    Add the updater plugin to the builder chain BEFORE `.setup(|app| {` (modify the chain, do NOT remove existing chain elements):
    ```rust
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::get_companion_state,
            commands::set_companion_prefs_cmd,
            commands::get_companion_prefs_cmd,
            // Phase 4 commands
            commands::rescan_paths,
            commands::install_pending_update,
            commands::wizard_dismiss,
            commands::open_settings_window,
        ])
        .setup(|app| { ... })
    ```

    Re-export Phase 4 types if useful for tests. The existing `pub use commands::{AppState, CompanionState};` line at end of `pub mod commands` block does NOT need change.
  </action>
  <verify>
    <automated>
      cd C:/Users/reema/Documents/Programming/achievements/src-tauri &amp;&amp;
      cargo build --lib --quiet 2>&amp;1 | tail -20 &amp;&amp;
      cargo build --bin hallmark --quiet 2>&amp;1 | tail -10 &amp;&amp;
      cargo test --lib --quiet store::queries::tests::first_run_done_round_trip store::queries::tests::last_update_check_round_trip store::queries::tests::first_run_done_isolated_from_completion 2>&amp;1 | grep -E '3 passed|test result.*ok' &amp;&amp;
      grep -c "pub mod tray\|pub mod autostart\|pub mod test_trigger\|pub mod first_run\|pub mod settings_window\|pub mod portable_mode\|pub mod updater_glue" src/lib.rs &amp;&amp;
      grep -q "raw_tx_for_state" src/lib.rs &amp;&amp;
      grep -q "tauri_plugin_updater::Builder" src/lib.rs &amp;&amp;
      grep -q "rescan_paths\|install_pending_update\|wizard_dismiss\|open_settings_window" src/lib.rs &amp;&amp;
      grep -q "test_trigger::seed_test_fixture" src/lib.rs &amp;&amp;
      grep -q "tray::build_tray" src/lib.rs &amp;&amp;
      grep -q "updater_glue::spawn_background_check" src/lib.rs &amp;&amp;
      grep -q "first_run::open_wizard" src/lib.rs
    </automated>
  </verify>
  <acceptance_criteria>
    - `cargo build --lib` succeeds with no errors. Warnings about unused imports/fields in stub modules are acceptable.
    - `cargo build --bin hallmark` succeeds.
    - 3 round-trip tests from 04-01a's queries.rs additions now pass (`cargo test --lib`).
    - Confirm 7 Phase 4 module declarations exist via 7 separate `pub mod {tray|autostart|test_trigger|first_run|settings_window|portable_mode|updater_glue};` lines.
    - `AppState` struct contains exactly 8 fields (3 existing + 5 new).
    - The `tauri::Builder` chain has `.plugin(tauri_plugin_updater::Builder::new().build())` BEFORE `.invoke_handler(...)`.
    - `tauri::generate_handler![...]` lists all 7 commands (3 existing + 4 new).
    - `setup()` calls `tray::build_tray`, `updater_glue::spawn_background_check`, `first_run::open_wizard`, `test_trigger::seed_test_fixture` — each in its prescribed location with surrounding error-warn-and-continue handling per PATTERNS.md "Error Handling".
    - On `cargo run --bin hallmark` (manual smoke test, NOT part of automated verify), startup logs show: portable_mode flag, silent_launch flag, `tray::build_tray STUB` warn, `updater_glue::spawn_background_check STUB` warn, `first_run::open_wizard STUB` warn or `wizard skipped` debug — proving the spine wires every entry point.
  </acceptance_criteria>
  <done>
    `lib.rs::run()` is the integration spine. Every Phase 4 entry point is called from setup(); each module's body is a stub that logs WARN. Wave 2 plans can swap stubs for real implementations without touching `lib.rs`.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| filesystem→app | `tauri.conf.json` is read by build tools only — no untrusted input |
| network→app | CSP `connect-src` defines outbound trust set — Phase 4 expands it for GitHub Releases |
| build→runtime | `bundle.createUpdaterArtifacts: true` produces .sig files signed at build time with a key not yet generated |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-04-04 | T (Tampering) | tauri.conf.json `pubkey` placeholder | mitigate | Plan 04-06 replaces `PLACEHOLDER_REPLACE_AT_RELEASE` with real Ed25519 public key generated via `tauri signer generate`. Until then, `bundle.active: true` will warn at build time but updater plugin will refuse signature verification at runtime — fail-safe (no malicious update can validate against a placeholder string). |
| T-04-05 | I (Information Disclosure) | CSP whitelist | accept | Adding `https://github.com` + `https://objects.githubusercontent.com` widens connect-src. These are Microsoft-operated CDN/origin and constitute the canonical OSS distribution surface; threat is bounded to "GitHub itself becomes hostile" which is out of scope for a hobby OSS project (every GitHub-distributed app shares this risk). |
| T-04-06 | E (Elevation of Privilege) | NSIS installMode | mitigate | `installMode: "perUser"` enforces no-UAC install — installer cannot elevate. D-22. |
| T-04-07 | I | `AppState.pending_update` Mutex | accept | A local attacker with code-execution-as-user already owns the process; protecting in-process state from such an attacker is out of scope per PROJECT.md local-only stance. |
</threat_model>

<verification>
- `cargo build --lib` and `cargo build --bin hallmark` succeed (cwd: src-tauri).
- `cargo test --lib store::queries::tests::first_run_done_round_trip store::queries::tests::last_update_check_round_trip store::queries::tests::first_run_done_isolated_from_completion` passes (cwd: src-tauri).
- `pnpm build` succeeds (cwd: repo root) — produces `dist/companion/`, `dist/popup/`, `dist/settings/`, `dist/wizard/` rollup outputs (still verifies that 04-01a's Vite config was correctly applied).
- `cargo tauri dev` smoke-launches and the existing Phase 1-3 popup pipeline still fires on a Goldberg unlock. (Manual; not gated on this plan's pass/fail since dev launch needs a Goldberg fixture.)
- `python -c "import json; json.load(open('src-tauri/tauri.conf.json'))"` exits 0.
</verification>

<success_criteria>
- tauri.conf.json bundle + plugins.updater + CSP land correctly.
- lib.rs declares 7 new modules, extends AppState to 8 fields, registers updater plugin, registers 4 new commands, and calls every Phase 4 entry point from setup().
- queries.rs round-trip tests (added in 04-01a) now pass.
- Wave 2 plans (04-02, 04-03, 04-04, 04-05) can each modify ONE module file (plus their own frontend files where applicable) without touching `lib.rs`, `Cargo.toml`, `tauri.conf.json`, `vite.config.ts`, or capability JSONs.
- No Phase 1-3 regression: existing Goldberg unlock pipeline still flows end-to-end.
</success_criteria>

<output>
After completion, create `.planning/phases/04-polish-distribution/04-01b-SUMMARY.md` summarizing:
- AppState shape now in effect (8 fields)
- Configuration deltas (bundle config, plugins.updater, CSP additions)
- 4 new Tauri commands registered
- Critical invariants Wave 2 plans must preserve (raw_tx clone semantics, pending_update Mutex async-friendliness, cached_discovery RwLock pattern)
- Pre-flight items for Plan 04-06 (Ed25519 keypair generation; pubkey placeholder location in tauri.conf.json)
</output>
</content>
