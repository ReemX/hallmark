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

use tauri::{Listener, Manager};
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

/// Production entry — invoked by `bin/main.rs`. Starts the Tauri shell with
/// Phase 2 subsystems wired:
///   • SQLite store (with 001 + 002 migrations).
///   • Path discovery (Phase 1) — uses real DiscoveredPaths fields + paths::goldberg_* helpers.
///   • Goldberg adapter + watcher + pipeline (Phase 1).
///   • Popup overlay + companion windows (Plan 05).
///   • SchemaCache + AudioDispatcher (Plans 02, 04).
///   • popup_queue + game_detect tokio tasks (Plans 03, 05).
///   • Tauri commands for companion data + prefs IO (Plan 06).
///   • game-started listener that hands off pid from Plan 03's payload to
///     popup_queue's current_pid mutex (POPUP-03 functional routing).
pub fn run() {
    init_tracing();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Hallmark starting (Phase 2 — full UI pipeline)"
    );

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_companion_state,
            commands::set_companion_prefs_cmd,
            commands::get_companion_prefs_cmd,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // ----- 1. Resolve user data dir + open store -----
            let db_dir = dirs::data_dir()
                .ok_or_else(|| anyhow::anyhow!("data_dir unavailable"))?
                .join("Hallmark");
            std::fs::create_dir_all(&db_dir)?;
            let db_path = db_dir.join("hallmark.db");
            let store = std::sync::Arc::new(store::SqliteStore::open(&db_path)?);
            tracing::info!(path = %db_path.display(), "store opened (001 + 002 migrations applied)");

            // ----- 2. Create session -----
            let session_id = uuid::Uuid::new_v4().to_string();
            store.with_conn(|c| store::queries::create_session(c, &session_id, None))?;
            tracing::info!(session_id = %session_id, "session created");

            // ----- 3. Path discovery — canonical DiscoveredPaths -----
            let discovery = paths::discover();
            tracing::info!(
                steam_libraries = discovery.steam_libraries.len(),
                goldberg_save_roots = discovery.goldberg_save_roots.len(),
                goldberg_redirects = discovery.goldberg_local_save_redirects.len(),
                "path discovery complete"
            );

            // ----- 4. Bind goldberg helpers BEFORE moving into closures (B-3 fix) -----
            // The struct does NOT have `.goldberg_roots` or `.redirect_map` fields —
            // those were fictional in the prior plan. Use the public helpers.
            let steam_libraries = discovery.steam_libraries.clone();
            let goldberg_paths = paths::goldberg_watch_paths(&discovery);
            let goldberg_map = paths::goldberg_redirect_map(&discovery);

            // ----- 5. Build adapter list -----
            let goldberg_adapter: std::sync::Arc<dyn sources::SourceAdapter> =
                std::sync::Arc::new(sources::goldberg::GoldbergAdapter::new(
                    goldberg_paths.clone(),
                    goldberg_map.clone(),
                ));
            let adapters = vec![goldberg_adapter];

            // ----- 6. Channels (cli mirrors this topology) -----
            let (raw_tx, raw_rx) = tokio::sync::mpsc::channel::<sources::RawUnlockEvent>(64);
            let (sink_tx, sink_rx) = tokio::sync::mpsc::channel::<sources::RawUnlockEvent>(64);

            // ----- 7. Audio dispatcher (best-effort; popups go silent if device unavailable) -----
            let audio_opt: Option<std::sync::Arc<audio::AudioDispatcher>> =
                match audio::AudioDispatcher::new() {
                    Ok(a) => Some(std::sync::Arc::new(a)),
                    Err(e) => {
                        tracing::warn!(error = %e, "audio init failed; popups will be visual-only this session");
                        None
                    }
                };

            // ----- 8. Schema cache -----
            let schema_cache = schema::SchemaCache::new(store.clone())?;

            // ----- 9. Windows -----
            ui::create_popup_window(&app_handle)?;
            ui::create_companion_window(&app_handle)?;
            tracing::info!("popup + companion windows created (hidden)");

            // ----- 10. AppState management -----
            app.manage(AppState {
                store: store.clone(),
                schema: schema_cache.clone(),
                session_id: session_id.clone(),
            });

            // ----- 11. Shared current_pid for popup placement -----
            let current_pid: std::sync::Arc<tokio::sync::Mutex<Option<u32>>> =
                std::sync::Arc::new(tokio::sync::Mutex::new(None));

            // ----- 12. Spawn pipeline tasks -----
            tokio::spawn(watcher::run_watcher(adapters, raw_tx));
            tracing::info!("spawned run_watcher");

            tokio::spawn(watcher::run_pipeline(
                raw_rx,
                store.clone(),
                session_id.clone(),
                sink_tx,
                std::time::Duration::from_secs(10),
            ));
            tracing::info!("spawned run_pipeline");

            if let Some(audio_arc) = audio_opt {
                let app_for_queue = app_handle.clone();
                let store_for_queue = store.clone();
                let session_for_queue = session_id.clone();
                let schema_for_queue = schema_cache.clone();
                let pid_for_queue = current_pid.clone();
                tokio::spawn(async move {
                    popup_queue::run(
                        app_for_queue, sink_rx, schema_for_queue, audio_arc,
                        store_for_queue, session_for_queue, pid_for_queue,
                    ).await;
                });
                tracing::info!("spawned popup_queue");
            } else {
                // Drain sink so run_pipeline doesn't backpressure when audio is unavailable.
                tokio::spawn(async move {
                    let mut rx = sink_rx;
                    while let Some(_) = rx.recv().await {
                        tracing::debug!("event drained (no audio device — popup_queue not started)");
                    }
                });
            }

            // ----- 13. game_detect task -----
            let app_for_detect = app_handle.clone();
            let store_for_detect = store.clone();
            let libraries_for_detect = steam_libraries.clone();
            let goldberg_for_detect = goldberg_map.clone();
            tokio::spawn(async move {
                game_detect::run(
                    app_for_detect, store_for_detect,
                    libraries_for_detect, goldberg_for_detect,
                ).await;
            });
            tracing::info!("spawned game_detect");

            // ----- 14. game-started listener: extract pid + write current_pid + spawn schema::resolve -----
            // The payload now carries BOTH app_id AND pid (Plan 03 B-1 fix). Plan 07
            // populates current_pid from this field so popup_queue routes popups to
            // the running game's monitor (POPUP-03 functional, not just helpers-exist).
            let pid_for_listener = current_pid.clone();
            let schema_for_listener = schema_cache.clone();
            let app_for_listener = app_handle.clone();
            let goldberg_redirect_for_listener = goldberg_map.clone();
            let _unlisten_started = app.listen("game-started", move |event: tauri::Event| {
                let payload: serde_json::Value = match serde_json::from_str(event.payload()) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to parse game-started payload");
                        return;
                    }
                };
                let Some(app_id) = payload.get("app_id").and_then(|v| v.as_u64()) else {
                    tracing::warn!("game-started payload missing app_id");
                    return;
                };
                // Plan 03 B-1 fix: payload.pid is present.
                let pid_opt = payload.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32);

                // Write pid into shared mutex so popup_queue's position_popup can
                // resolve the game's HWND on the next fire (POPUP-03 functional routing).
                if let Some(pid) = pid_opt {
                    let pid_handle = pid_for_listener.clone();
                    tokio::spawn(async move {
                        let mut guard = pid_handle.lock().await;
                        *guard = Some(pid);
                        tracing::info!(app_id, pid, "current_pid updated for popup placement");
                    });
                } else {
                    tracing::warn!(app_id, "game-started payload missing pid; popup falls back to last-set position");
                }

                // Spawn schema resolution per D-25.
                let schema_clone = schema_for_listener.clone();
                let app_clone = app_for_listener.clone();
                // Find Goldberg JSON paths for this app_id.
                let goldberg_paths_for_app: Vec<std::path::PathBuf> =
                    goldberg_redirect_for_listener.iter()
                        .filter(|(_, gid)| **gid == app_id)
                        .map(|(path, _)| path.join("achievements.json"))
                        .collect();
                tokio::spawn(async move {
                    tracing::info!(app_id, count = goldberg_paths_for_app.len(), "starting schema resolve");
                    schema_clone.resolve(app_clone, app_id, goldberg_paths_for_app).await;
                });
            });

            tracing::info!("Phase 2 setup complete; all subsystems running");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri runtime failed to start");
}
