//! Hallmark library entry point.
//!
//! Phase 1 scope: tracing initialization, Tauri builder skeleton with empty `windows`
//! array, and a `setup()` hook that LATER plans (04 watcher, 05 dedup+cli) attach
//! background tokio tasks to. This file establishes the structure; downstream plans
//! extend `setup()` rather than restructuring.

pub mod paths;
pub mod sources;
pub mod store;
pub mod watcher;

// ---- Phase 2 modules ----
// Each module is a stub placeholder until its owning plan populates the body.
// Lifting these declarations into Plan 01 prevents lib.rs file conflicts in Wave 2.
pub mod schema;        // Plan 02 — D-24 lookup chain, AchievementSchema, classify_tier
pub mod audio;         // Plan 04 — rodio AudioDispatcher
pub mod monitor;       // Plan 03 — Win32 HWND-by-PID + monitor placement
pub mod popup_queue;   // Plan 05 — drain task with adaptive compression + 100% rule
pub mod ui;            // Plan 05 — popup + companion WebviewWindowBuilder + HWND patch
pub mod game_detect;   // Plan 03 — sysinfo + Steam state hybrid + appmanifest match

use tracing_subscriber::EnvFilter;

// ============================================================================
// Phase 2 Plan 06: Tauri commands for companion window data fetch + prefs IO.
// Plan 07's setup() registers these via tauri::generate_handler! and manages AppState.
// Commands are placed in a sub-module to avoid proc-macro name-collision in crate root.
// ============================================================================

pub mod commands {
    use std::collections::HashMap;
    use std::sync::Arc;
    use serde::{Deserialize, Serialize};

    /// Application-wide state shared with Tauri command handlers via `tauri::State`.
    /// Plan 07 constructs this in `setup()` and registers via `app.manage(state)`.
    pub struct AppState {
        pub store: Arc<crate::store::SqliteStore>,
        pub schema: crate::schema::SchemaCache,
        pub session_id: String,
    }

    /// Snapshot of one game's companion view: full achievement schema + earned map.
    /// `earned` maps ach_api_name → unlocked_at unix epoch (i64), filtered to the
    /// current session_id (COMP-03 mid-restart restore reads from this).
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CompanionState {
        pub app_id: u64,
        pub schema: Vec<crate::schema::AchievementSchema>,
        pub earned: HashMap<String, i64>,
        pub session_id: String,
    }

    #[tauri::command]
    pub fn get_companion_state(
        app_id: u64,
        state: tauri::State<'_, AppState>,
    ) -> Result<CompanionState, String> {
        let session_id = state.session_id.clone();
        let schema_list = state.schema.list_for_app(app_id);
        let earned = state.store.with_conn(|c| -> anyhow::Result<HashMap<String, i64>> {
            let app_id_i64 = i64::try_from(app_id)?;
            let mut stmt = c.prepare(
                "SELECT ach_api_name, unlocked_at FROM unlock_history
                 WHERE app_id = ?1 AND session_id = ?2"
            )?;
            let rows = stmt.query_map(rusqlite::params![app_id_i64, session_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?;
            let mut map = HashMap::new();
            for r in rows { let (k, v) = r?; map.insert(k, v); }
            Ok(map)
        }).map_err(|e| e.to_string())?;
        Ok(CompanionState { app_id, schema: schema_list, earned, session_id })
    }

    #[tauri::command]
    pub fn set_companion_prefs_cmd(
        prefs: crate::store::queries::CompanionPrefs,
        state: tauri::State<'_, AppState>,
    ) -> Result<(), String> {
        state.store.with_conn(|c| crate::store::queries::set_companion_prefs(c, &prefs))
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn get_companion_prefs_cmd(
        app_id: u64,
        state: tauri::State<'_, AppState>,
    ) -> Result<Option<crate::store::queries::CompanionPrefs>, String> {
        state.store.with_conn(|c| crate::store::queries::get_companion_prefs(c, app_id))
            .map_err(|e| e.to_string())
    }
}

// Re-export top-level types for Plan 07 convenience.
pub use commands::{AppState, CompanionState};

/// Initialize structured logging. Call once at process start.
/// Reads RUST_LOG env var; defaults to `hallmark_lib=info,warn` for clean output.
///
/// WR-07: If `try_init` fails (e.g. a global subscriber already installed),
/// surface the error to stderr rather than silently dropping it. Production
/// `init_tracing()` should be called exactly once; a second call indicates a bug.
/// Tests that need to tolerate repeat calls should use `init_tracing_for_tests()`.
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

/// Tests-only: initialize tracing if not already initialized, swallowing the
/// "already installed" error explicitly. Multiple `#[tokio::test]`s in one
/// process will both call this; only the first does anything useful.
///
/// Made `pub` so integration tests in `tests/` can call it. Marked
/// `#[allow(dead_code)]` because not every test invokes it.
#[allow(dead_code)]
pub fn init_tracing_for_tests() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("hallmark_lib=warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_level(true)
        .try_init();
}

/// Production entry — invoked by `bin/main.rs`. Starts the Tauri shell.
///
/// Phase 1: Tauri starts but creates NO windows. This is configured in
/// `tauri.conf.json` via `app.windows = []` and `app.security.csp = null`,
/// both of which are **intentional for Phase 1's headless backend** (IN-06).
/// Phase 2 will add the popup overlay window and a CSP appropriate to it.
/// The process stays alive via Tauri's run loop; Plans 04/05 spawn background
/// tasks inside the `setup()` closure.
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
            tracing::info!(
                "Tauri setup complete (no background tasks attached in Phase 1 scaffold)"
            );
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri runtime failed to start");
}
