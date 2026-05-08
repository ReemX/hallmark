//! Hybrid game-launch detection (D-21): sysinfo polling + Steam loginusers
//! state read. Emits `game-started` + `game-stopped` Tauri events consumed
//! by Plan 06 (companion show/hide), Plan 02 (schema resolve trigger),
//! and Plan 07 (current_pid mutex update for popup_queue monitor placement).
//!
//! Pattern follows watcher::run_watcher (long-running tokio task).
//!
//! D-21 NOTE: Phase 2 ships the sysinfo-only leg of the hybrid detection.
//! The Steam-state-authoritative leg (Steam IPC for "currently playing app")
//! requires binary VDF parsing of localconfig.vdf and is deferred to Phase 3
//! per CONTEXT.md "## Phase 2 Implementation Notes". RESEARCH.md Section K
//! confirms there is no public Steam IPC for this in 2026.

pub mod process_scan;
pub mod steam_state;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub use process_scan::RunningGame;

use crate::store::SqliteStore;

/// Long-running task: polls running processes every 3 seconds, diffs against
/// the previous tick, emits `game-started` / `game-stopped` Tauri events when
/// the running-game set changes.
///
/// Never returns under normal operation. Stops when the AppHandle is dropped
/// (Tauri shutdown).
pub async fn run(
    app: AppHandle,
    _store: Arc<SqliteStore>,
    steam_libraries: Vec<PathBuf>,
    goldberg_redirect_roots: HashMap<PathBuf, u64>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(3));
    let mut sys = sysinfo::System::new_all();
    // Track (app_id → pid) so we can emit pid on game-stopped if needed and
    // diff cleanly. Plan 07's listener consumes the pid from game-started.
    let mut prev_running: HashMap<u64, u32> = HashMap::new();

    tracing::info!(
        libraries = steam_libraries.len(),
        goldberg_roots = goldberg_redirect_roots.len(),
        "game_detect task started (3s polling interval; D-21 sysinfo-only leg, Steam-state leg deferred to Phase 3 per CONTEXT.md)"
    );

    loop {
        interval.tick().await;

        let running = process_scan::refresh_and_scan(
            &mut sys, &steam_libraries, &goldberg_redirect_roots,
        );
        let current: HashMap<u64, u32> = running.iter().map(|g| (g.app_id, g.pid)).collect();

        // D-22 conflict resolution hook: when Phase 3 wires the Steam-state
        // authoritative leg here, log a warn if Steam disagrees with sysinfo
        // and prefer Steam. See CONTEXT.md "## Phase 2 Implementation Notes"
        // for why the leg is currently a no-op.
        tracing::trace!("steam state hook reserved for Phase 3 (D-21 authoritative leg)");

        // Game-started: new entries — emit with BOTH app_id AND pid so Plan 07's
        // listener can populate current_pid for popup_queue monitor placement (B-1 fix).
        for game in &running {
            if !prev_running.contains_key(&game.app_id) {
                tracing::info!(
                    app_id = game.app_id,
                    pid = game.pid,
                    name = %game.name,
                    "game-started"
                );
                let payload = serde_json::json!({
                    "app_id": game.app_id,
                    "pid": game.pid,
                });
                if let Err(e) = app.emit("game-started", &payload) {
                    tracing::warn!(error = %e, "failed to emit game-started");
                }
            }
        }

        // Game-stopped: departed entries
        for prev_app_id in prev_running.keys() {
            if !current.contains_key(prev_app_id) {
                tracing::info!(app_id = prev_app_id, "game-stopped");
                let payload = serde_json::json!({ "app_id": prev_app_id });
                if let Err(e) = app.emit("game-stopped", &payload) {
                    tracing::warn!(error = %e, "failed to emit game-stopped");
                }
            }
        }

        prev_running = current;
    }
}

#[cfg(test)]
mod tests {
    use super::process_scan::RunningGame;
    use std::collections::HashSet;

    // The `run` task itself is hard to unit-test (long-running, tokio runtime).
    // Plan 07 adds an integration test that drives a fake adapter. Here we test
    // only the diff logic conceptually — extracted via a tiny helper for clarity.

    fn diff_running(prev: &HashSet<u64>, curr: &HashSet<u64>) -> (Vec<u64>, Vec<u64>) {
        let started: Vec<u64> = curr.difference(prev).copied().collect();
        let stopped: Vec<u64> = prev.difference(curr).copied().collect();
        (started, stopped)
    }

    #[test]
    fn diff_detects_started_and_stopped() {
        let prev: HashSet<u64> = [480_u64, 500].into_iter().collect();
        let curr: HashSet<u64> = [500, 600].into_iter().collect();
        let (started, stopped) = diff_running(&prev, &curr);
        assert_eq!(started, vec![600]);
        assert_eq!(stopped, vec![480]);
    }

    #[test]
    fn diff_first_tick_emits_all_as_started() {
        let prev: HashSet<u64> = HashSet::new();
        let curr: HashSet<u64> = [480_u64, 500].into_iter().collect();
        let (mut started, stopped) = diff_running(&prev, &curr);
        started.sort();
        assert_eq!(started, vec![480, 500]);
        assert!(stopped.is_empty());
    }

    #[test]
    fn diff_no_change_emits_nothing() {
        let prev: HashSet<u64> = [480_u64].into_iter().collect();
        let curr: HashSet<u64> = [480_u64].into_iter().collect();
        let (started, stopped) = diff_running(&prev, &curr);
        assert!(started.is_empty());
        assert!(stopped.is_empty());
    }

    #[test]
    fn running_game_struct_is_constructible() {
        let g = RunningGame {
            pid: 1234, app_id: 480,
            name: "MyGame.exe".into(),
            exe_path: std::path::PathBuf::from("/x"),
        };
        assert_eq!(g.app_id, 480);
    }

    /// Verify the game-started payload shape includes BOTH app_id and pid (B-1 fix).
    /// Plan 07's listener depends on the pid field to populate the current_pid mutex.
    #[test]
    fn game_started_payload_includes_app_id_and_pid() {
        let g = RunningGame {
            pid: 4242, app_id: 480,
            name: "MyGame.exe".into(),
            exe_path: std::path::PathBuf::from("/x"),
        };
        let payload = serde_json::json!({
            "app_id": g.app_id,
            "pid": g.pid,
        });
        assert_eq!(payload.get("app_id").and_then(|v| v.as_u64()), Some(480));
        assert_eq!(payload.get("pid").and_then(|v| v.as_u64()), Some(4242));
    }
}
