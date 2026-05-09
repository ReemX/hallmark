---
phase: 04-polish-distribution
plan: 01a
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/Cargo.toml
  - src-tauri/capabilities/companion.json
  - src-tauri/capabilities/settings.json
  - src-tauri/capabilities/wizard.json
  - src-tauri/src/tray.rs
  - src-tauri/src/autostart.rs
  - src-tauri/src/test_trigger.rs
  - src-tauri/src/portable_mode.rs
  - src-tauri/src/first_run.rs
  - src-tauri/src/settings_window.rs
  - src-tauri/src/updater_glue.rs
  - src-tauri/src/store/queries.rs
  - package.json
  - vite.config.ts
  - settings.html
  - wizard.html
  - src/types.ts
autonomous: true
requirements:
  - POL-01
  - POL-02
  - DIST-01
  - DIST-02
  - DIST-04
tags:
  - tauri
  - phase4-foundation-a
  - module-scaffolding

must_haves:
  truths:
    - "Cargo build succeeds with `tauri-plugin-updater = \"2.10\"` added"
    - "Frontend build succeeds with `@tauri-apps/plugin-updater@^2` and 4 Vite entry points (companion, popup, settings, wizard)"
    - "settings.html and wizard.html exist as Vite entry points and reference `/src/main-settings.tsx` and `/src/main-wizard.tsx` respectively"
    - "All 7 Phase 4 module files exist (tray, autostart, test_trigger, first_run, settings_window, portable_mode, updater_glue) as compilable stubs that log WARN at runtime"
    - "queries.rs exposes `get_first_run_done`, `set_first_run_done`, `get_last_update_check`, `set_last_update_check` with passing round-trip tests"
    - "src/types.ts exports `SourceStatus`, `DiscoveredPathsView`, `UpdateInfo`, `FirstRunState` for downstream React pages"
    - "Existing Phase 1-3 code is unchanged byte-for-byte (only ADDs to queries.rs; new files for the 7 modules)"
  artifacts:
    - path: "src-tauri/Cargo.toml"
      provides: "tauri-plugin-updater 2.10 dependency"
      contains: 'tauri-plugin-updater'
    - path: "src-tauri/src/tray.rs"
      provides: "Module stub — body in Plan 04-02"
    - path: "src-tauri/src/autostart.rs"
      provides: "Module stub — body in Plan 04-02"
    - path: "src-tauri/src/test_trigger.rs"
      provides: "Module stub — body in Plan 04-03"
    - path: "src-tauri/src/portable_mode.rs"
      provides: "Module stub — body in Plan 04-03 (is_silent_launch implemented now)"
    - path: "src-tauri/src/settings_window.rs"
      provides: "Module stub — body in Plan 04-04"
    - path: "src-tauri/src/updater_glue.rs"
      provides: "Module stub — body in Plan 04-04"
    - path: "src-tauri/src/first_run.rs"
      provides: "Module stub — body in Plan 04-05"
    - path: "src-tauri/capabilities/settings.json"
      provides: "Settings window capability"
    - path: "src-tauri/capabilities/wizard.json"
      provides: "Wizard window capability"
    - path: "settings.html"
      provides: "Vite entry for Settings window"
    - path: "wizard.html"
      provides: "Vite entry for Wizard window"
    - path: "vite.config.ts"
      provides: "4-entry rollup config"
      contains: "settings: resolve"
    - path: "src-tauri/src/store/queries.rs"
      provides: "Phase 4 settings persistence helpers + 3 round-trip tests"
      contains: 'get_first_run_done'
  key_links:
    - from: "settings.html / wizard.html"
      to: "src/main-settings.tsx / src/main-wizard.tsx"
      via: "module script tag"
      pattern: 'src="/src/main-(settings|wizard)\\.tsx"'
    - from: "src-tauri/src/store/queries.rs"
      to: "settings table key namespacing"
      via: "INSERT OR REPLACE WHERE key='first_run_done'|'last_update_check'"
      pattern: "'first_run_done'|'last_update_check'"
---

<objective>
Foundation A of the Phase 4 integration spine. This plan introduces all
dependency adds, Vite multi-entry config, capability JSONs, frontend HTML
entries, the 7 Phase 4 module file stubs, the queries.rs persistence helpers,
and the types.ts extension. After this plan completes, all the **scaffolding
files** Wave 2 plans need exist and compile — but `lib.rs` itself is NOT yet
modified. Plan 04-01b consumes these stubs and wires them into `lib.rs::run()`.

Purpose: Halve the original 04-01 plan's context cost (18 files → ~10 files
here, ~8 files in 04-01b). 04-01a is purely additive — every artifact is a new
file or an append to `Cargo.toml`/`package.json`/`vite.config.ts`/`queries.rs`.
No semantic dependency between this plan and `lib.rs` extension; the stubs
expose function signatures that 04-01b will call.

Output: A buildable codebase whose Phase 4 modules are stubs (logging WARN at
runtime), whose Vite build produces 4 entry bundles, and whose queries.rs is
ready for the wizard / updater settings reads. The Phase 1-3 pipeline is
unchanged.
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
@CLAUDE.md

@src-tauri/Cargo.toml
@src-tauri/capabilities/companion.json
@src-tauri/src/lib.rs
@src-tauri/src/store/queries.rs
@src-tauri/src/sources/mod.rs
@src-tauri/src/paths.rs
@vite.config.ts
@package.json
@popup.html
@index.html

<interfaces>
<!-- Existing types Wave 2 plans will work against (unchanged by this plan; embedded
     here so executors don't need to grep). -->

From src-tauri/src/sources/mod.rs:
```rust
pub struct RawUnlockEvent {
    pub app_id: u64,
    pub ach_api_name: String,
    pub timestamp: u64,
    pub source: SourceKind,
}
pub enum SourceKind { Goldberg, SteamLegit, CreamApi, SmartSteamEmu }
```

From src-tauri/src/lib.rs (existing AppState — DO NOT modify in this plan;
04-01b extends it):
```rust
pub struct AppState {
    pub store: Arc<crate::store::SqliteStore>,
    pub schema: crate::schema::SchemaCache,
    pub session_id: String,
}
```

From src-tauri/src/paths.rs:
```rust
pub struct DiscoveredPaths {
    pub steam_install: Option<PathBuf>,
    pub steam_libraries: Vec<PathBuf>,
    pub goldberg_save_roots: Vec<PathBuf>,
    pub goldberg_local_save_redirects: Vec<GoldbergRedirect>,
    pub steam_legit_appcache_stats: Option<PathBuf>,
    pub steam_legit_user_ids: Vec<u64>,
    pub cream_api_appid_dirs: Vec<PathBuf>,
    pub sse_appid_dirs: Vec<PathBuf>,
}
```

From src-tauri/src/store/queries.rs (lines 85-110 — existing pattern to mirror):
```rust
pub fn mark_completion_fired(conn: &Connection, app_id: u64) -> anyhow::Result<()> { ... }
pub fn is_completion_fired(conn: &Connection, app_id: u64) -> anyhow::Result<bool> { ... }
```

</interfaces>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Cargo + frontend deps + Vite multi-entry + capability JSONs + HTML entries</name>
  <files>
    src-tauri/Cargo.toml,
    package.json,
    vite.config.ts,
    settings.html,
    wizard.html,
    src-tauri/capabilities/settings.json,
    src-tauri/capabilities/wizard.json,
    src-tauri/capabilities/companion.json
  </files>
  <read_first>
    src-tauri/Cargo.toml,
    package.json,
    vite.config.ts,
    popup.html,
    index.html,
    src-tauri/capabilities/companion.json,
    src-tauri/capabilities/popup.json,
    .planning/phases/04-polish-distribution/04-RESEARCH.md (sections "Standard Stack", "Pattern 8", "Tauri capability for Settings window", "Tauri capability for first-run wizard window"),
    .planning/phases/04-polish-distribution/04-PATTERNS.md (sections "src-tauri/capabilities/settings.json", "src-tauri/capabilities/wizard.json", "src-tauri/capabilities/companion.json (extended)", "Pattern 8 Vite multi-entry config")
  </read_first>
  <action>
    Add `tauri-plugin-updater = "2.10"` to `[dependencies]` in `src-tauri/Cargo.toml` directly under the existing `reqwest` line, with a comment `# Phase 4: auto-updater (DIST-02)`. Do NOT bump or alter any other dependency version. `winreg = "0.56"` is already pinned under `[target.'cfg(target_os = "windows")'.dependencies]` — leave unchanged. Verify by running `cargo metadata --format-version 1 --no-deps -q | grep tauri-plugin-updater`.

    Add `"@tauri-apps/plugin-updater": "^2"` to `dependencies` in `package.json` (alphabetic order: place between `@tauri-apps/api` and `framer-motion`). Run `pnpm install` if executor's environment supports it; otherwise the build verification in Task 3 confirms the lock.

    Create `settings.html` at repo root mirroring `popup.html` exactly. Title: `Hallmark Settings`. Stylesheet href: `/src/styles/settings.css`. Module script src: `/src/main-settings.tsx`. (CSS file is created in Plan 04-04; for this plan the href is a forward reference and Vite tolerates it because it bundles by importer.)

    Create `wizard.html` at repo root mirroring `popup.html`. Title: `Welcome to Hallmark`. Stylesheet href: `/src/styles/settings.css` (UI-SPEC permits wizard reusing settings.css). Module script src: `/src/main-wizard.tsx`.

    Extend `vite.config.ts` `rollupOptions.input` to four entries — preserve the exact resolve()/__dirname pattern Phase 2 used:
    ```typescript
    input: {
      companion: resolve(__dirname, "index.html"),
      popup: resolve(__dirname, "popup.html"),
      settings: resolve(__dirname, "settings.html"),
      wizard: resolve(__dirname, "wizard.html"),
    },
    ```

    Create `src-tauri/capabilities/settings.json` per RESEARCH § "Tauri capability for Settings window":
    ```json
    {
      "$schema": "../gen/schemas/desktop-schema.json",
      "identifier": "settings-capability",
      "description": "Settings window — read-only paths panel, update check, about. Custom drag region.",
      "windows": ["settings"],
      "permissions": [
        "core:default",
        "core:event:allow-listen",
        "core:event:allow-unlisten",
        "core:window:allow-show",
        "core:window:allow-hide",
        "core:window:allow-close",
        "core:window:allow-start-dragging",
        "updater:default"
      ]
    }
    ```

    Create `src-tauri/capabilities/wizard.json` per RESEARCH § "Tauri capability for first-run wizard window" (NO `updater:default` — wizard does not invoke updater):
    ```json
    {
      "$schema": "../gen/schemas/desktop-schema.json",
      "identifier": "wizard-capability",
      "description": "First-run welcome wizard. Shows path-discovery results.",
      "windows": ["wizard"],
      "permissions": [
        "core:default",
        "core:event:allow-listen",
        "core:event:allow-unlisten",
        "core:window:allow-show",
        "core:window:allow-close",
        "core:window:allow-start-dragging"
      ]
    }
    ```

    Extend `src-tauri/capabilities/companion.json` `permissions` array to add `"updater:default"` (D-18 modal Install button calls `install_pending_update` which is a backend command, but the modal also imports from `@tauri-apps/plugin-updater` for type definitions — capability is required). Append at the end of the array, comma-separated. Do NOT remove or reorder existing permissions.
  </action>
  <verify>
    <automated>
      cd C:/Users/reema/Documents/Programming/achievements &amp;&amp;
      grep -q '^tauri-plugin-updater\s*=\s*"2.10"' src-tauri/Cargo.toml &amp;&amp;
      grep -q '"@tauri-apps/plugin-updater":\s*"\^2"' package.json &amp;&amp;
      test -f settings.html &amp;&amp; test -f wizard.html &amp;&amp;
      grep -q 'settings:\s*resolve' vite.config.ts &amp;&amp;
      grep -q 'wizard:\s*resolve' vite.config.ts &amp;&amp;
      test -f src-tauri/capabilities/settings.json &amp;&amp;
      test -f src-tauri/capabilities/wizard.json &amp;&amp;
      grep -q '"updater:default"' src-tauri/capabilities/settings.json &amp;&amp;
      grep -q '"updater:default"' src-tauri/capabilities/companion.json &amp;&amp;
      ! grep -q '"installMode"' src-tauri/capabilities/wizard.json &amp;&amp;
      ! grep -q 'updater' src-tauri/capabilities/wizard.json
    </automated>
  </verify>
  <acceptance_criteria>
    - `cargo metadata` (run in `src-tauri/`) lists `tauri-plugin-updater` 2.10.x as a dependency.
    - `pnpm install` resolves `@tauri-apps/plugin-updater` ^2 (or `pnpm-lock.yaml` is updated to include it).
    - `settings.html` and `wizard.html` exist at repo root and reference their respective .tsx entries.
    - `vite.config.ts` `rollupOptions.input` has exactly 4 keys: companion, popup, settings, wizard.
    - `src-tauri/capabilities/settings.json` contains `"identifier": "settings-capability"` and `"updater:default"`.
    - `src-tauri/capabilities/wizard.json` contains `"identifier": "wizard-capability"` and does NOT contain `"updater:default"` or `"installMode"`.
    - `src-tauri/capabilities/companion.json` `permissions` array contains `"updater:default"`.
  </acceptance_criteria>
  <done>
    All deps + capability + Vite entries land. No new Tauri commands or Rust modules added in this task — Task 2 handles Rust modules.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Module file stubs (7 files) + types.ts extension</name>
  <files>
    src-tauri/src/tray.rs,
    src-tauri/src/autostart.rs,
    src-tauri/src/test_trigger.rs,
    src-tauri/src/portable_mode.rs,
    src-tauri/src/first_run.rs,
    src-tauri/src/settings_window.rs,
    src-tauri/src/updater_glue.rs,
    src/types.ts
  </files>
  <read_first>
    src/types.ts (full),
    src-tauri/src/lib.rs (lines 8-21 module ladder — for current module list; do NOT modify),
    .planning/phases/04-polish-distribution/04-RESEARCH.md (Pattern 6 lines 654-712, Common Pitfalls #4 `--silent` parsing),
    .planning/phases/04-polish-distribution/04-PATTERNS.md (sections "Recommended Project Structure" lines 268-323, "Established Patterns")
  </read_first>
  <behavior>
    - Test 1 (`is_silent_launch_in_test_runner`): cargo test runs without `--silent`, so `portable_mode::is_silent_launch()` returns false. Confirms the function exists and the heuristic is correct.
  </behavior>
  <action>
    Create the 7 module file stubs. Each is a minimal, COMPILING placeholder that Wave 2 plans replace. The stubs MUST expose the function signatures Wave 2 plans implement — this fixes the contract surface so `lib.rs::run()` (04-01b) can call them.

    NOTE: this task creates the FILES only. It does NOT add `pub mod` declarations to `lib.rs` — that is 04-01b's job. The files must therefore compile as orphans (no `super::` references). The canonical verification is the full `cargo build --lib` AFTER 04-01b adds the `pub mod` lines.

    `src-tauri/src/tray.rs`:
    ```rust
    //! Tray icon + menu — Phase 4 Plan 04-02 owns implementation.
    //! See CONTEXT.md D-01 for menu structure, D-02 for icon presence,
    //! D-03 for Quit semantics, D-09 for autostart-toggle state sync.

    use tauri::App;

    /// Build and register the system-tray icon. Plan 04-02 implements.
    #[allow(unused_variables)]
    pub fn build_tray(app: &App) -> tauri::Result<()> {
        tracing::warn!("tray::build_tray STUB — Plan 04-02 not yet implemented");
        Ok(())
    }
    ```

    `src-tauri/src/autostart.rs` (cross-platform stubs — windows-only impl in Plan 04-02):
    ```rust
    //! HKCU\Run autostart helper — Phase 4 Plan 04-02 owns implementation.
    //! See CONTEXT.md D-07 (HKCU only, never HKLM), D-08 (--silent flag).

    /// Read live HKCU\Run state for the "Hallmark" value.
    pub fn is_enabled() -> anyhow::Result<bool> {
        tracing::warn!("autostart::is_enabled STUB — Plan 04-02 not yet implemented");
        Ok(false)
    }

    /// Write `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Hallmark`.
    pub fn enable() -> anyhow::Result<()> {
        tracing::warn!("autostart::enable STUB — Plan 04-02 not yet implemented");
        Ok(())
    }

    /// Remove the "Hallmark" value from HKCU\Run (does NOT delete the key itself).
    pub fn disable() -> anyhow::Result<()> {
        tracing::warn!("autostart::disable STUB — Plan 04-02 not yet implemented");
        Ok(())
    }
    ```

    `src-tauri/src/test_trigger.rs`:
    ```rust
    //! Synthetic RawUnlockEvent test trigger — Phase 4 Plan 04-03 owns implementation.
    //! See CONTEXT.md D-04..D-06.

    use tauri::AppHandle;

    pub const TEST_API_NAME: &str = "HALLMARK_TEST_UNLOCK";
    pub const TEST_APP_ID: u64 = 480;  // Spacewar — Steam test app

    /// Inject a synthetic unlock event at the adapter→dedup boundary (D-04).
    /// Plan 04-03 implements; Plan 04-02's tray menu calls this on "Fire test popup".
    #[allow(unused_variables)]
    pub fn fire(app: &AppHandle) -> anyhow::Result<()> {
        tracing::warn!("test_trigger::fire STUB — Plan 04-03 not yet implemented");
        Ok(())
    }

    /// Pre-seed schema_cache with the test fixture row (D-05). Plan 04-03 implements;
    /// `lib.rs::run()` calls this once after schema_cache is constructed (04-01b).
    #[allow(unused_variables)]
    pub fn seed_test_fixture(store: &crate::store::SqliteStore) -> anyhow::Result<()> {
        tracing::warn!("test_trigger::seed_test_fixture STUB — Plan 04-03 not yet implemented");
        Ok(())
    }
    ```

    `src-tauri/src/portable_mode.rs`:
    ```rust
    //! Portable-vs-installed-mode detection + --silent argv parsing.
    //! Phase 4 Plan 04-03 owns implementation. See CONTEXT.md D-08, D-23.

    /// True if the running exe is NOT inside `%LOCALAPPDATA%\Hallmark`.
    pub fn is_portable() -> bool {
        tracing::warn!("portable_mode::is_portable STUB — Plan 04-03 not yet implemented; defaulting to non-portable");
        false
    }

    /// True if the process was launched with `--silent` (HKCU\Run autostart).
    pub fn is_silent_launch() -> bool {
        std::env::args().any(|a| a == "--silent")
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn is_silent_launch_in_test_runner() {
            // cargo test does not pass --silent
            assert!(!is_silent_launch());
        }
    }
    ```
    Note: `is_silent_launch` is a one-liner (RESEARCH Pitfall 4) so it's implemented now — no need to defer to Plan 04-03.

    `src-tauri/src/first_run.rs`:
    ```rust
    //! First-run wizard window builder + flag-aware open logic.
    //! Phase 4 Plan 04-05 owns implementation. See CONTEXT.md D-13..D-17.

    use tauri::AppHandle;

    /// Build and show the wizard window when the first-run-done flag is unset
    /// OR when 0 paths are detected (D-14 re-fire logic).
    /// Plan 04-05 implements.
    #[allow(unused_variables)]
    pub fn open_wizard(app: AppHandle, any_path_detected: bool) -> tauri::Result<()> {
        tracing::warn!("first_run::open_wizard STUB — Plan 04-05 not yet implemented");
        Ok(())
    }
    ```

    `src-tauri/src/settings_window.rs`:
    ```rust
    //! Settings window builder — Phase 4 Plan 04-04 owns implementation.
    //! See CONTEXT.md D-10, D-11, D-12.

    use tauri::AppHandle;

    /// Open (or re-focus if already open) the Settings window.
    /// Plan 04-04 implements.
    #[allow(unused_variables)]
    pub fn open(app: &AppHandle) -> tauri::Result<()> {
        tracing::warn!("settings_window::open STUB — Plan 04-04 not yet implemented");
        Ok(())
    }
    ```

    `src-tauri/src/updater_glue.rs`:
    ```rust
    //! tauri-plugin-updater background-check + AppState pending-update stash.
    //! Phase 4 Plan 04-04 owns implementation. See CONTEXT.md D-18..D-21.

    use tauri::AppHandle;

    /// Background-check `latest.json` on startup. If newer version available,
    /// stash on AppState.pending_update and emit `update-available` to companion.
    /// Skipped silently when `portable_mode::is_portable()` returns true (D-23).
    /// Plan 04-04 implements.
    #[allow(unused_variables)]
    pub fn spawn_background_check(app: AppHandle) {
        tracing::warn!("updater_glue::spawn_background_check STUB — Plan 04-04 not yet implemented");
    }
    ```

    Extend `src/types.ts` (append after existing `SchemaResolvedPayload`):
    ```typescript
    /** Phase 4 — surfaces of DiscoveredPaths to Settings + Wizard React pages. */
    export interface SourceStatus {
      name: "Steam" | "Goldberg" | "CreamAPI" | "SmartSteamEmu";
      found: boolean;
      detail?: string; // e.g. "libraryfolders.vdf not found"
    }
    export interface DiscoveredPathsView {
      sources: SourceStatus[];
    }

    /** Phase 4 — UpdateModal payload. Mirrors `tauri_plugin_updater::Update` subset. */
    export interface UpdateInfo {
      version: string;
      notes: string | null;
    }

    /** Phase 4 — first-run wizard payload. */
    export interface FirstRunState {
      sources: SourceStatus[];
      any_found: boolean;
    }
    ```
  </action>
  <verify>
    <automated>
      cd C:/Users/reema/Documents/Programming/achievements &amp;&amp;
      test -f src-tauri/src/tray.rs &amp;&amp;
      test -f src-tauri/src/autostart.rs &amp;&amp;
      test -f src-tauri/src/test_trigger.rs &amp;&amp;
      test -f src-tauri/src/portable_mode.rs &amp;&amp;
      test -f src-tauri/src/first_run.rs &amp;&amp;
      test -f src-tauri/src/settings_window.rs &amp;&amp;
      test -f src-tauri/src/updater_glue.rs &amp;&amp;
      grep -q "STUB" src-tauri/src/tray.rs &amp;&amp;
      grep -q "STUB" src-tauri/src/autostart.rs &amp;&amp;
      grep -q "STUB" src-tauri/src/test_trigger.rs &amp;&amp;
      grep -q "STUB" src-tauri/src/first_run.rs &amp;&amp;
      grep -q "STUB" src-tauri/src/settings_window.rs &amp;&amp;
      grep -q "STUB" src-tauri/src/updater_glue.rs &amp;&amp;
      grep -q "is_silent_launch" src-tauri/src/portable_mode.rs &amp;&amp;
      grep -q "SourceStatus" src/types.ts &amp;&amp;
      grep -q "DiscoveredPathsView" src/types.ts &amp;&amp;
      grep -q "UpdateInfo" src/types.ts &amp;&amp;
      grep -q "FirstRunState" src/types.ts
    </automated>
  </verify>
  <acceptance_criteria>
    - All 7 module files exist with the stub bodies above.
    - Each stub function (except `portable_mode::is_silent_launch`) logs WARN at runtime.
    - `portable_mode::is_silent_launch()` is fully implemented (RESEARCH Pitfall 4 one-liner).
    - `src/types.ts` exports `SourceStatus`, `DiscoveredPathsView`, `UpdateInfo`, `FirstRunState`.
    - No existing types in `types.ts` are removed or modified.
    - Files do NOT use `super::` references (must compile when included via `pub mod` in 04-01b).
  </acceptance_criteria>
  <done>
    Module stubs land. Wave 2 plans can each fill in one module's body without touching anything else.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: queries.rs first_run + last_update_check helpers + 3 round-trip tests</name>
  <files>src-tauri/src/store/queries.rs</files>
  <read_first>
    src-tauri/src/store/queries.rs (full — focus on lines 85-110 `is_completion_fired` / `mark_completion_fired` for the pattern; lines 196-269 for the test pattern),
    src-tauri/src/store/migrations/001_initial.sql (full — confirm settings(key TEXT PRIMARY KEY, value TEXT) schema),
    .planning/phases/04-polish-distribution/04-PATTERNS.md (section "src-tauri/src/store/queries.rs (extended)")
  </read_first>
  <behavior>
    - Test 1 (`first_run_done_round_trip`): on a fresh in-memory store, `get_first_run_done` returns `Ok(false)`. After `set_first_run_done`, it returns `Ok(true)`. After `set_first_run_done` called twice in a row, it still returns `Ok(true)` (idempotent INSERT OR REPLACE).
    - Test 2 (`last_update_check_round_trip`): on a fresh in-memory store, `get_last_update_check` returns `Ok(None)`. After `set_last_update_check(c, 1715000000)`, it returns `Ok(Some(1715000000))`. After `set_last_update_check(c, 1715999999)`, it returns `Ok(Some(1715999999))` (overwrite, not append).
    - Test 3 (`first_run_done_isolated_from_completion`): writing `mark_completion_fired(c, 480)` does NOT cause `get_first_run_done(c)` to return `true` (key namespacing — `completion_<app_id>` vs `first_run_done`).
  </behavior>
  <action>
    Append to `src-tauri/src/store/queries.rs` (do NOT remove or alter existing code) — locate the closing of the existing test module and insert the four new functions BEFORE the `#[cfg(test)]` block:

    ```rust
    // ============================================================================
    // Phase 4 additions: first-run wizard flag + updater last-checked timestamp.
    // Settings table (key, value) reused — same pattern as completion_<app_id>.
    // No new migration; key='first_run_done' value='1' (D-14).
    //                  key='last_update_check' value='<unix_secs as string>'.
    // ============================================================================

    /// Read the first-run-done flag (D-14). Returns `false` when the row is absent
    /// (fresh install) — wizard fires.
    pub fn get_first_run_done(conn: &Connection) -> anyhow::Result<bool> {
        let result = conn.query_row(
            "SELECT value FROM settings WHERE key = 'first_run_done'",
            [],
            |r| r.get::<_, String>(0),
        );
        match result {
            Ok(v) => Ok(v == "1"),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// Write the first-run-done flag (D-14). Idempotent. Caller is responsible
    /// for the dismiss-with-≥1-path predicate per CONTEXT.md D-14.
    pub fn set_first_run_done(conn: &Connection) -> anyhow::Result<()> {
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('first_run_done', '1')",
            [],
        )?;
        Ok(())
    }

    /// Read the last successful update-check unix timestamp. Returns Ok(None)
    /// on first run or after DB wipe. Used by Settings → Updates panel for
    /// "Last checked: {relative time or 'just now'}" per UI-SPEC.
    pub fn get_last_update_check(conn: &Connection) -> anyhow::Result<Option<i64>> {
        let result = conn.query_row(
            "SELECT value FROM settings WHERE key = 'last_update_check'",
            [],
            |r| r.get::<_, String>(0),
        );
        match result {
            Ok(v) => Ok(v.parse::<i64>().ok()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Write the last successful update-check unix timestamp.
    pub fn set_last_update_check(conn: &Connection, unix_secs: i64) -> anyhow::Result<()> {
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('last_update_check', ?1)",
            params![unix_secs.to_string()],
        )?;
        Ok(())
    }
    ```

    Append the three round-trip tests to the existing `#[cfg(test)] mod tests` block at end of file (use the existing `fresh_store()` helper):

    ```rust
    #[test]
    fn first_run_done_round_trip() {
        let s = fresh_store();
        s.with_conn(|c| {
            assert_eq!(get_first_run_done(c).unwrap(), false, "fresh DB has no first_run_done flag");
            set_first_run_done(c).unwrap();
            assert_eq!(get_first_run_done(c).unwrap(), true, "after set, flag is true");
            set_first_run_done(c).unwrap();
            assert_eq!(get_first_run_done(c).unwrap(), true, "set is idempotent");
            Ok(())
        }).unwrap();
    }

    #[test]
    fn last_update_check_round_trip() {
        let s = fresh_store();
        s.with_conn(|c| {
            assert_eq!(get_last_update_check(c).unwrap(), None, "fresh DB has no timestamp");
            set_last_update_check(c, 1715000000).unwrap();
            assert_eq!(get_last_update_check(c).unwrap(), Some(1715000000));
            set_last_update_check(c, 1715999999).unwrap();
            assert_eq!(get_last_update_check(c).unwrap(), Some(1715999999), "overwrite, not append");
            Ok(())
        }).unwrap();
    }

    #[test]
    fn first_run_done_isolated_from_completion() {
        let s = fresh_store();
        s.with_conn(|c| {
            mark_completion_fired(c, 480).unwrap();
            assert_eq!(get_first_run_done(c).unwrap(), false, "completion key does not affect first_run_done");
            Ok(())
        }).unwrap();
    }
    ```

    NOTE: `cargo build --lib` for the **whole crate** is NOT run here because the 7 module files from Task 2 do not yet have `pub mod` declarations in `lib.rs` — that is 04-01b's job. The 3 new tests will run successfully after 04-01b adds the module ladder lines.
  </action>
  <verify>
    <automated>
      cd C:/Users/reema/Documents/Programming/achievements/src-tauri &amp;&amp;
      grep -q "get_first_run_done" src/store/queries.rs &amp;&amp;
      grep -q "set_first_run_done" src/store/queries.rs &amp;&amp;
      grep -q "get_last_update_check" src/store/queries.rs &amp;&amp;
      grep -q "set_last_update_check" src/store/queries.rs &amp;&amp;
      grep -q "first_run_done_round_trip" src/store/queries.rs &amp;&amp;
      grep -q "last_update_check_round_trip" src/store/queries.rs &amp;&amp;
      grep -q "first_run_done_isolated_from_completion" src/store/queries.rs
    </automated>
  </verify>
  <acceptance_criteria>
    - `src-tauri/src/store/queries.rs` contains the 4 new public functions and 3 new tests.
    - 04-01b verification (after lib.rs `pub mod` lines added) confirms `cargo test --lib store::queries::tests::first_run_done_round_trip` passes (and the other two).
    - No existing tests in `queries.rs` are removed or modified.
  </acceptance_criteria>
  <done>
    queries.rs persistence helpers ship with tests. Plan 04-01b will add `pub mod` declarations to lib.rs and run the full build.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| filesystem→app | `Cargo.toml`, `package.json`, `vite.config.ts` are read by build tools only — no untrusted input |
| build→runtime | New module files contain stub functions only — no behavior changes |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-04-01 | T (Tampering) | settings table key namespacing | mitigate | New keys (`first_run_done`, `last_update_check`) namespaced to avoid collision with existing `completion_<app_id>` rows. Round-trip test `first_run_done_isolated_from_completion` proves the isolation. |
| T-04-02 | I (Information Disclosure) | new capability JSONs | accept | settings/wizard capabilities only grant window + event + updater permissions. No new file/shell/path access. |
| T-04-03 | T | module stubs accidentally fire side effects | mitigate | Every stub body returns Ok early after a tracing::warn. No production logic invoked until Wave 2 plans replace them. |
</threat_model>

<verification>
- `cargo metadata --format-version 1 --no-deps -q` (cwd: `src-tauri/`) lists `tauri-plugin-updater 2.10.x`.
- `pnpm install` exits 0 (or `pnpm-lock.yaml` already includes the new entry).
- `python -c "import json; json.load(open('src-tauri/capabilities/settings.json'))"` exits 0.
- `python -c "import json; json.load(open('src-tauri/capabilities/wizard.json'))"` exits 0.
- File existence checks for the 7 stub module files + settings.html + wizard.html.
- 3 new test functions present in queries.rs (live test execution gated on 04-01b adding the module ladder).
</verification>

<success_criteria>
- All Phase 4 stub module files exist and are orphan-compilable (no `super::` deps).
- queries.rs has 4 new helpers + 3 new tests appended without modifying any existing code.
- Capability JSONs land with correct identifiers and permission lists.
- Vite multi-entry config has 4 entries.
- types.ts exports the 4 Phase 4 type interfaces.
- 04-01b can now extend `lib.rs` to add `pub mod` declarations + AppState fields + setup() wiring.
</success_criteria>

<output>
After completion, create `.planning/phases/04-polish-distribution/04-01a-SUMMARY.md` summarizing:
- Files added (count of new modules + new HTML entries + new capabilities)
- Configuration deltas (deps added, Vite entries, capability identifiers)
- queries.rs delta (4 helpers + 3 tests)
- Pre-flight for 04-01b: lib.rs needs `pub mod` declarations for the 7 new modules + AppState extension + setup() integration spine
</output>
</content>
