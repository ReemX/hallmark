//! tauri-plugin-updater background-check + AppState pending-update stash.
//! D-18: emit "update-available" event so React companion shows modal on next open.
//! D-19: stable channel only; latest.json from GitHub Releases.
//! D-23: skipped when portable_mode.
//!
//! Phase 4 gap-closure (04-12): manual_check + spawn_background_check now
//! distinguish 4 error categories at the FFI boundary so the UI can render
//! accurate copy. See .planning/debug/updates-error-wording-misleading.md.

use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

/// Tagged enum returned to the frontend by the manual update check.
/// serde-tagged so JS can switch on `result.status`.
///
/// Variants:
///   * Available    — newer version found; `version` + `notes` populated; pending_update stashed
///   * UpToDate     — already on the latest version
///   * NoReleaseYet — `latest.json` returned non-2xx (most likely 404 because no v0.1.0 published yet)
///   * Offline      — DNS / TCP / TLS / timeout — network unreachable
///   * PlatformMissing — release exists but does not advertise this platform
///   * OtherError   — any other Error variant; `detail` carries the underlying message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CheckOutcome {
    Available { version: String, notes: Option<String> },
    UpToDate,
    NoReleaseYet,
    Offline { detail: String },
    PlatformMissing { detail: String },
    OtherError { detail: String },
}

/// Internal categorisation of `tauri_plugin_updater::Error`. Public for unit
/// testing — the real `Error` variants are `#[non_exhaustive]` and not all
/// can be cheaply constructed in tests, so the test surface uses this enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckErrorKind {
    ReleaseNotFound,
    ReqwestNetwork,    // is_connect / is_timeout / DNS / TLS
    TargetNotFound,
    Other,
}

/// Categorise a tauri_plugin_updater::Error. Pure function for testability.
fn classify_check_error(e: &tauri_plugin_updater::Error) -> CheckErrorKind {
    use tauri_plugin_updater::Error as PUError;
    match e {
        PUError::ReleaseNotFound => CheckErrorKind::ReleaseNotFound,
        PUError::Reqwest(_) => CheckErrorKind::ReqwestNetwork,
        PUError::TargetNotFound(_) | PUError::TargetsNotFound(_) => CheckErrorKind::TargetNotFound,
        _ => CheckErrorKind::Other,
    }
}

/// Map a categorised error + the underlying message to a CheckOutcome.
fn map_kind_to_outcome(kind: CheckErrorKind, detail: String) -> CheckOutcome {
    match kind {
        CheckErrorKind::ReleaseNotFound => CheckOutcome::NoReleaseYet,
        CheckErrorKind::ReqwestNetwork => CheckOutcome::Offline { detail },
        CheckErrorKind::TargetNotFound => CheckOutcome::PlatformMissing { detail },
        CheckErrorKind::Other => CheckOutcome::OtherError { detail },
    }
}

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
            Err(e) => {
                // Phase 4 gap-closure (04-12): differentiate log level by kind.
                // - ReleaseNotFound (404 — no release published yet) is INFO-level
                //   because it's the expected state for a fresh repo; not a failure.
                // - Reqwest (network) is WARN-level — actual transport issue.
                // - Other variants WARN-level too for triage visibility.
                let kind = classify_check_error(&e);
                match kind {
                    CheckErrorKind::ReleaseNotFound => {
                        tracing::info!(error = %e, "update check: no release published yet (expected for fresh repo)");
                        // Treat as a successful "checked" event for UX freshness.
                        persist_last_checked(&app);
                    }
                    _ => {
                        tracing::warn!(error = %e, kind = ?kind, "update check failed");
                    }
                }
            }
        }
    });
}

/// Manual Settings → "Check for Updates" button entry point.
/// Returns a CheckOutcome (tagged enum) so the frontend can render
/// kind-specific copy. The outer Result::Err is reserved for unrecoverable
/// bugs only (updater plugin missing entirely).
pub async fn manual_check(app: AppHandle) -> Result<CheckOutcome, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            let notes: Option<String> = update.body.clone();
            let state = app.state::<crate::AppState>();
            let mut guard = state.pending_update.lock().await;
            *guard = Some(update);
            drop(guard);
            persist_last_checked(&app);
            Ok(CheckOutcome::Available { version, notes })
        }
        Ok(None) => {
            persist_last_checked(&app);
            Ok(CheckOutcome::UpToDate)
        }
        Err(e) => {
            let detail = e.to_string();
            let kind = classify_check_error(&e);
            let outcome = map_kind_to_outcome(kind.clone(), detail.clone());
            // For NoReleaseYet, persist last-checked so the UI shows
            // "Last checked: just now" — the check succeeded as a check;
            // the absence of a release is the answer, not a failure.
            if matches!(kind, CheckErrorKind::ReleaseNotFound) {
                persist_last_checked(&app);
            }
            Ok(outcome)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_kind_release_not_found() {
        let outcome = map_kind_to_outcome(CheckErrorKind::ReleaseNotFound, "ignored".into());
        assert!(matches!(outcome, CheckOutcome::NoReleaseYet));
    }

    #[test]
    fn map_kind_reqwest_network() {
        let detail = "DNS error".to_string();
        let outcome = map_kind_to_outcome(CheckErrorKind::ReqwestNetwork, detail.clone());
        match outcome {
            CheckOutcome::Offline { detail: d } => assert_eq!(d, "DNS error"),
            _ => panic!("expected Offline; got {outcome:?}"),
        }
    }

    #[test]
    fn map_kind_target_not_found() {
        let outcome = map_kind_to_outcome(CheckErrorKind::TargetNotFound, "win-x64 missing".into());
        assert!(matches!(outcome, CheckOutcome::PlatformMissing { .. }));
    }

    #[test]
    fn map_kind_other_preserves_detail() {
        let outcome = map_kind_to_outcome(CheckErrorKind::Other, "weird parse error".into());
        match outcome {
            CheckOutcome::OtherError { detail } => assert_eq!(detail, "weird parse error"),
            _ => panic!("expected OtherError; got {outcome:?}"),
        }
    }

    #[test]
    fn check_outcome_serializes_with_snake_case_tag() {
        let outcome = CheckOutcome::NoReleaseYet;
        let json = serde_json::to_string(&outcome).unwrap();
        assert!(json.contains(r#""status":"no_release_yet""#),
            "expected snake_case tag 'no_release_yet'; got {json}");

        let avail = CheckOutcome::Available {
            version: "0.1.1".into(),
            notes: Some("First release".into()),
        };
        let json = serde_json::to_string(&avail).unwrap();
        assert!(json.contains(r#""status":"available""#));
        assert!(json.contains(r#""version":"0.1.1""#));
    }

    #[test]
    fn check_outcome_round_trip() {
        let outcome = CheckOutcome::Offline { detail: "timeout".into() };
        let json = serde_json::to_string(&outcome).unwrap();
        let parsed: CheckOutcome = serde_json::from_str(&json).unwrap();
        match parsed {
            CheckOutcome::Offline { detail } => assert_eq!(detail, "timeout"),
            _ => panic!("round-trip failed; got {parsed:?}"),
        }
    }
}
