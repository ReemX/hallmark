//! tauri-plugin-updater background-check + AppState pending-update stash.
//! D-18: emit "update-available" event so React companion shows modal on next open.
//! D-19: stable channel only; latest.json from GitHub Releases.
//! D-23: skipped when portable_mode.

use std::time::SystemTime;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

/// Background-check on startup. Stashes any pending Update on AppState
/// and emits "update-available" with {version, notes} to the companion window.
pub fn spawn_background_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let updater = match app.updater() {
            Ok(u) => u,
            Err(e) => {
                tracing::warn!(error = %e, "updater not available; skipping check");
                return;
            }
        };
        match updater.check().await {
            Ok(Some(update)) => {
                let version = update.version.clone();
                // tauri_plugin_updater::Update.body is Option<String> per docs.rs (2.10).
                // Default empty when latest.json omits notes — treat as JSON null in payload.
                let notes: Option<String> = update.body.clone();
                tracing::info!(version = %version, "update available — stashing for modal");

                // Stash on AppState
                let state = app.state::<crate::AppState>();
                {
                    let mut guard = state.pending_update.lock().await;
                    *guard = Some(update);
                }

                // Emit to companion (it may or may not be visible yet — D-18 says modal
                // appears on next companion open, so the companion's listener stashes
                // the event payload until then).
                let payload = serde_json::json!({
                    "version": version,
                    "notes": notes,
                });
                let _ = app.emit_to("companion", "update-available", payload);

                // Persist last-checked timestamp.
                persist_last_checked(&app);
            }
            Ok(None) => {
                tracing::info!("update check: already on latest version");
                persist_last_checked(&app);
            }
            Err(e) => tracing::warn!(error = %e, "update check failed"),
        }
    });
}

/// Manual Settings → "Check for Updates" button entry point.
/// Returns version-and-notes if newer, or None if up to date.
pub async fn manual_check(app: AppHandle) -> Result<Option<crate::commands::UpdateInfoView>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update_opt = updater.check().await.map_err(|e| e.to_string())?;
    persist_last_checked(&app);
    match update_opt {
        Some(update) => {
            let version = update.version.clone();
            // body: Option<String> per tauri-plugin-updater 2.10 docs.
            let notes: Option<String> = update.body.clone();
            let state = app.state::<crate::AppState>();
            let mut guard = state.pending_update.lock().await;
            *guard = Some(update);
            Ok(Some(crate::commands::UpdateInfoView { version, notes }))
        }
        None => Ok(None),
    }
}

fn persist_last_checked(app: &AppHandle) {
    let now = match SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(_) => return,
    };
    let state = app.state::<crate::AppState>();
    if let Err(e) = state.store.with_conn(|c| crate::store::queries::set_last_update_check(c, now)) {
        tracing::warn!(error = %e, "set_last_update_check failed");
    }
}
