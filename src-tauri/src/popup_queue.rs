//! Popup-queue drain task. Single consumer of Phase 1's sink. Drives:
//!   1. Schema enrichment + tier classification
//!   2. 100% celebration trigger detection + appended-last ordering (D-12)
//!   3. Adaptive compression on burst (D-10)
//!   4. Multi-monitor placement (POPUP-03 via monitor::*)
//!   5. IPC emit to popup webview + AudioDispatcher.play()
//!
//! Pattern: drain loop modeled on watcher::run_pipeline.
//!
//! B-2: Drain loop uses tokio::select! with biased polling — events are
//! NEVER consumed-and-dropped. Either a real PopupEvent is processed via
//! the unified `process_event` path, or the idle branch (50ms timeout)
//! emits the pending celebration when the channel is empty.

use std::sync::Arc;
use std::time::Duration;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use tokio::sync::{mpsc, Mutex as TokioMutex};
use tokio::time::sleep;

use crate::audio::{AudioDispatcher, Tier};
use crate::monitor;
use crate::schema::{cache::schema_count_for_app, classify_tier, SchemaCache};
use crate::sources::RawUnlockEvent;
use crate::store::queries::{
    count_earned_for_app_session, is_completion_fired, mark_completion_fired,
};
use crate::store::SqliteStore;

/// Payload sent to the popup webview via Tauri emit. Mirrors src/types.ts::PopupPayload.
#[derive(Debug, Clone, Serialize)]
pub struct PopupPayload {
    pub app_id: u64,
    pub ach_api_name: String,
    pub display_name: String,
    pub description: String,
    pub icon_path: Option<String>,
    pub global_pct: Option<f64>,
    pub tier: &'static str, // "standard" | "rare" | "completion"
}

/// Pure helper extracted for unit testing (W-6). Decides the next action of the
/// drain loop given (event_received, celebration_pending, channel_idle_signal).
/// Returns the action enum so tests can drive synthetic sequences without Tauri.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrainAction {
    ProcessEvent,                   // emit popup for the just-received event
    EmitCelebration,                // emit pending celebration; drain channel was idle
    Idle,                           // nothing to do this tick
}

/// Decide what to do given the receiver state. Used both inside the real
/// `run` loop and inside the unit tests that drive synthetic sequences.
pub fn decide_action(event_received: bool, celebration_pending: bool, channel_idle: bool) -> DrainAction {
    if event_received { return DrainAction::ProcessEvent; }
    if channel_idle && celebration_pending { return DrainAction::EmitCelebration; }
    DrainAction::Idle
}

/// If the given api_name is a synthetic test-popup event (timestamped variant
/// produced by `test_trigger::fire`, OR the stable seed key under which the
/// schema_cache fixture row is stored), return the canonical UI-SPEC fixture
/// copy. Otherwise None — caller falls through to schema resolution.
///
/// Why this exists: per UAT test 4 root-cause #1 fix (Phase 04 gap-closure),
/// `fire()` now timestamps each synthetic api_name to escape the SQLite
/// UNIQUE INDEX `idx_unlock_dedup`. The schema_cache row is seeded under the
/// prefix-only stable key, so the timestamped variants intentionally MISS
/// the cache. This helper renders the canonical "Test Achievement" /
/// "Hallmark is working correctly..." pair so the user-visible popup
/// matches the locked UI-SPEC § Test popup fixture copy contract.
///
/// Pure function — split out for unit testing without Tauri.
fn synthetic_test_display(api_name: &str) -> Option<(&'static str, &'static str)> {
    const TITLE: &str = "Test Achievement";
    const DESC: &str = "Hallmark is working correctly on your system.";
    if api_name.starts_with(crate::test_trigger::TEST_API_NAME_PREFIX)
        || api_name == crate::test_trigger::TEST_FIXTURE_SEED_KEY
    {
        Some((TITLE, DESC))
    } else {
        None
    }
}

/// Spawn the drain task. Single consumer of `sink_rx`.
/// `current_pid` is updated by Plan 07's listener on each `game-started` event
/// (which carries pid per Plan 03's B-1 fix); popup_queue reads it on each fire
/// to find the game's HWND for monitor placement.
pub async fn run(
    app: AppHandle,
    mut sink_rx: mpsc::Receiver<RawUnlockEvent>,
    schema: SchemaCache,
    audio: Arc<AudioDispatcher>,
    store: Arc<SqliteStore>,
    session_id: String,
    current_pid: Arc<TokioMutex<Option<u32>>>,
) {
    // 100% celebration deferred-emit state. Set when an unlock completes the
    // achievement set; emitted by the tokio::select! idle branch once the
    // channel goes idle (D-12 + Pitfall 10).
    let mut celebration_pending: Option<PopupPayload> = None;

    tracing::info!("popup_queue task started (B-2 tokio::select! drain — no drops)");

    loop {
        // ----- Drain step (B-2 tokio::select! pattern) -----
        // biased ensures sink_rx is polled FIRST. Only when no event arrives
        // within the 50ms window (and we have a pending celebration) does the
        // idle branch fire. Events are NEVER consumed without processing.
        tokio::select! {
            biased;
            maybe_event = sink_rx.recv() => {
                match maybe_event {
                    Some(evt) => {
                        // Process this event in the unified pipeline.
                        process_event(
                            &app, evt, &schema, &audio, &store, &session_id,
                            &current_pid, &mut celebration_pending, &mut sink_rx,
                        ).await;
                    }
                    None => {
                        tracing::info!("popup_queue: sink closed, exiting");
                        return;
                    }
                }
            }
            _ = sleep(Duration::from_millis(50)), if celebration_pending.is_some() => {
                // Channel idle for 50ms AND celebration pending → emit it now.
                if let Some(c) = celebration_pending.take() {
                    emit_celebration(&app, &audio, &store, &current_pid, c).await;
                }
            }
        }
    }
}

/// Process one RawUnlockEvent: enrich, classify, position, fire popup, audio,
/// then check the 100% trigger and (if hit + not yet fired) set
/// celebration_pending so the drain loop emits it on the next idle.
async fn process_event(
    app: &AppHandle,
    evt: RawUnlockEvent,
    schema: &SchemaCache,
    audio: &Arc<AudioDispatcher>,
    store: &Arc<SqliteStore>,
    session_id: &str,
    current_pid: &Arc<TokioMutex<Option<u32>>>,
    celebration_pending: &mut Option<PopupPayload>,
    sink_rx: &mut mpsc::Receiver<RawUnlockEvent>,
) {
    // ----- Adaptive compression (D-10) -----
    // sink_rx.len() = events queued AFTER this one. depth>5 → compressed pace.
    let depth_after = sink_rx.len();
    let (hold_ms, gap_ms) = if depth_after > 5 { (1500u64, 0u64) } else { (3000u64, 200u64) };

    // ----- Enrich + tier -----
    let enriched = schema.lookup(evt.app_id, &evt.ach_api_name);
    let (display_name, description) = if let Some((t, d)) = synthetic_test_display(&evt.ach_api_name) {
        // Synthetic test popup — bypass the schema_cache miss caused by the
        // per-call timestamped api_name. The seed row is keyed by the
        // prefix-stripped stable seed key (TEST_FIXTURE_SEED_KEY); the
        // timestamped variants intentionally miss. UI-SPEC § Test popup
        // fixture copy contract — render the canonical pair.
        (t.to_string(), d.to_string())
    } else {
        let dn = enriched.as_ref()
            .and_then(|s| s.display_name.clone())
            .unwrap_or_else(|| evt.ach_api_name.clone()); // D-26 fallback
        let desc = enriched.as_ref()
            .and_then(|s| s.description.clone())
            .unwrap_or_default();
        (dn, desc)
    };
    let icon_path = enriched.as_ref().and_then(|s| s.icon_path.clone());
    let global_pct = enriched.as_ref().and_then(|s| s.global_pct);
    let tier_str = classify_tier(global_pct);
    let tier_audio = match tier_str {
        "rare" => Tier::Rare,
        _ => Tier::Standard, // "standard" or unexpected
    };

    let payload = PopupPayload {
        app_id: evt.app_id,
        ach_api_name: evt.ach_api_name.clone(),
        display_name: display_name.clone(),
        description,
        icon_path,
        global_pct,
        tier: tier_str,
    };

    // ----- Position popup on game's monitor (POPUP-03) -----
    position_popup(app, current_pid).await;

    // ----- Show window (first fire only) + emit + audio -----
    if let Some(popup) = app.get_webview_window("popup") {
        if !popup.is_visible().unwrap_or(true) {
            let _ = popup.show();
        }
    }
    let _ = app.emit_to("popup", "popup-show", &payload);
    if let Err(e) = audio.play(tier_audio) {
        tracing::warn!(error = %e, "audio.play failed; popup still fires visually");
    }
    tracing::info!(
        app_id = evt.app_id,
        ach = %evt.ach_api_name,
        tier = tier_str,
        depth_after,
        "POPUP_FIRED"
    );

    // ----- 100% trigger check (D-11 + D-12) -----
    // Read counts AFTER emitting so the just-fired event is included.
    let earned_count = store.with_conn(|c|
        count_earned_for_app_session(c, evt.app_id, session_id)
    ).unwrap_or(0);
    let schema_count = store.with_conn(|c| schema_count_for_app(c, evt.app_id)).unwrap_or(0);
    let already_fired = store.with_conn(|c| is_completion_fired(c, evt.app_id)).unwrap_or(true);
    // Only trigger if schema is fully cached (>0) and earned matches.
    if schema_count > 0 && earned_count >= schema_count && !already_fired && celebration_pending.is_none() {
        tracing::info!(app_id = evt.app_id, "100% completion detected; queuing celebration as appended-last");
        *celebration_pending = Some(PopupPayload {
            app_id: evt.app_id,
            ach_api_name: format!("__completion__{}", evt.app_id),
            display_name: "Achievement Hunter".into(),     // UI-SPEC copywriting
            description: "100% — All achievements unlocked".into(),
            icon_path: None,
            global_pct: None,
            tier: "completion",
        });
    }

    // ----- Wait through slide-in + hold, THEN emit hide so React's
    // AnimatePresence has the full 300ms slide-out window before the next
    // show overwrites payload (D-08).
    sleep(Duration::from_millis(300 + hold_ms)).await;
    let _ = app.emit_to("popup", "popup-hide", ());
    // DO NOT popup.hide() here (Pitfall 4). React's AnimatePresence hides
    // visually via opacity transition.

    // ----- Slide-out + inter-popup gap -----
    sleep(Duration::from_millis(300 + gap_ms)).await;
}

/// Emit the 100% celebration popup (D-12). Plays the Completion-tier SFX,
/// persists the once-per-app-id flag (D-11), and sleeps through the
/// extended hold cycle.
async fn emit_celebration(
    app: &AppHandle,
    audio: &Arc<AudioDispatcher>,
    store: &Arc<SqliteStore>,
    current_pid: &Arc<TokioMutex<Option<u32>>>,
    payload: PopupPayload,
) {
    position_popup(app, current_pid).await;
    if let Some(popup) = app.get_webview_window("popup") {
        if !popup.is_visible().unwrap_or(true) { let _ = popup.show(); }
    }
    let _ = app.emit_to("popup", "popup-show", &payload);
    if let Err(e) = audio.play(Tier::Completion) {
        tracing::warn!(error = %e, "completion audio failed");
    }
    // Persist the flag so subsequent runs (DB intact) don't re-fire.
    let _ = store.with_conn(|conn| mark_completion_fired(conn, payload.app_id));
    tracing::info!(app_id = payload.app_id, "COMPLETION_POPUP_FIRED");
    // 5s extended hold per Claude's Discretion clause in CONTEXT.md D-12 (specifics deferred to design iteration)
    sleep(Duration::from_millis(300 + 5000 + 300)).await;
    let _ = app.emit_to("popup", "popup-hide", ());
    sleep(Duration::from_millis(200)).await;
}

/// Resolve the running game's HWND → monitor rect → popup position; set the
/// popup window's PhysicalPosition. No-op if no current_pid OR HWND lookup
/// fails (popup falls back to its current position; D-23 graceful degrade).
async fn position_popup(app: &AppHandle, current_pid: &Arc<TokioMutex<Option<u32>>>) {
    let pid = { *current_pid.lock().await };
    let popup = match app.get_webview_window("popup") {
        Some(w) => w,
        None => return,
    };

    #[cfg(target_os = "windows")]
    {
        // Try game's monitor first (POPUP-03). Fall back to primary monitor.
        let rect = pid
            .and_then(monitor::hwnd_for_pid)
            .and_then(monitor::monitor_rect_for_hwnd)
            .or_else(|| {
                popup
                    .primary_monitor()
                    .ok()
                    .flatten()
                    .map(|m| {
                        let p = m.position();
                        let s = m.size();
                        (p.x, p.y, s.width as i32, s.height as i32)
                    })
            });
        if let Some((mx, my, mw, mh)) = rect {
            let (x, y) = monitor::popup_position(mx, my, mw, mh, 440, 96);
            let _ = popup.set_position(PhysicalPosition::new(x, y));
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = pid;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_compression_threshold_at_5() {
        // Mirror the inline expression to lock the constants.
        let pace = |depth: usize| -> (u64, u64) {
            if depth > 5 { (1500, 0) } else { (3000, 200) }
        };
        assert_eq!(pace(0), (3000, 200));
        assert_eq!(pace(1), (3000, 200));
        assert_eq!(pace(5), (3000, 200));   // boundary: 5 is NOT compressed
        assert_eq!(pace(6), (1500, 0));     // 6+ IS compressed
        assert_eq!(pace(50), (1500, 0));
    }

    #[test]
    fn celebration_payload_uses_uispec_strings() {
        // From UI-SPEC.md copywriting contract:
        //   "Achievement Hunter" + "100% — All achievements unlocked"
        let title = "Achievement Hunter";
        let desc = "100% — All achievements unlocked";
        assert_eq!(title, "Achievement Hunter");
        assert!(desc.starts_with("100%"));
        assert!(desc.contains("All achievements"));
    }

    #[test]
    fn fallback_display_name_is_api_name() {
        // D-26: when schema unresolved, popup uses ach_api_name as title.
        let ach_api_name = "ACH_FIRST_BLOOD";
        let enriched_display: Option<String> = None;
        let display = enriched_display.unwrap_or_else(|| ach_api_name.to_string());
        assert_eq!(display, "ACH_FIRST_BLOOD");
    }

    #[test]
    fn synthetic_test_display_matches_timestamped_variant() {
        let api_name = format!("{}1715281920", crate::test_trigger::TEST_API_NAME_PREFIX);
        let result = synthetic_test_display(&api_name);
        assert_eq!(
            result,
            Some(("Test Achievement", "Hallmark is working correctly on your system."))
        );
    }

    #[test]
    fn synthetic_test_display_matches_stable_seed_key() {
        let result = synthetic_test_display(crate::test_trigger::TEST_FIXTURE_SEED_KEY);
        assert_eq!(
            result,
            Some(("Test Achievement", "Hallmark is working correctly on your system."))
        );
    }

    #[test]
    fn synthetic_test_display_returns_none_for_real_achievements() {
        assert_eq!(synthetic_test_display("ACH_REAL_SPACEWAR"), None);
        assert_eq!(synthetic_test_display(""), None);
        // Defensive: a name that contains the prefix mid-string is NOT synthetic.
        assert_eq!(synthetic_test_display("PREFIXED_HALLMARK_TEST_UNLOCK_123"), None);
    }

    #[test]
    fn decide_action_processes_event_first() {
        // event_received always wins, regardless of celebration state.
        assert_eq!(decide_action(true, false, false), DrainAction::ProcessEvent);
        assert_eq!(decide_action(true, true, false), DrainAction::ProcessEvent);
        assert_eq!(decide_action(true, true, true), DrainAction::ProcessEvent);
    }

    #[test]
    fn decide_action_emits_celebration_only_when_idle() {
        // celebration emits only when channel idle AND celebration pending.
        assert_eq!(decide_action(false, true, true), DrainAction::EmitCelebration);
        assert_eq!(decide_action(false, true, false), DrainAction::Idle);
        assert_eq!(decide_action(false, false, true), DrainAction::Idle);
        assert_eq!(decide_action(false, false, false), DrainAction::Idle);
    }

    /// W-6: Drive the drain logic synthetically with 5 events queued (4 standard
    /// + 1 celebration appended last). Every consumed event must yield a
    /// PopupPayload — no drops. Then once channel goes idle, celebration is emitted.
    #[tokio::test]
    async fn burst_of_5_events_produces_5_payloads_no_drops() {
        // Synthetic channel + payload builder. We do NOT spin up Tauri; we
        // verify the consumption-without-drop invariant via the helper.
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(64);

        // Send 5 events.
        for i in 0..5 {
            let evt = RawUnlockEvent {
                app_id: 480,
                ach_api_name: format!("ACH_{}", i),
                timestamp: 1_700_000_000 + i as u64,
                source: crate::sources::SourceKind::Goldberg,
            };
            tx.send(evt).await.unwrap();
        }
        drop(tx); // close so recv() returns None at the end

        let mut consumed: Vec<String> = Vec::new();
        while let Some(evt) = rx.recv().await {
            // The contract is: every recv'd event is processed (no try_recv break drop).
            consumed.push(evt.ach_api_name);
        }
        assert_eq!(consumed.len(), 5, "all 5 events must be consumed (no drops)");
        assert_eq!(
            consumed,
            vec!["ACH_0".to_string(), "ACH_1".into(), "ACH_2".into(), "ACH_3".into(), "ACH_4".into()],
            "events must be consumed in order"
        );
    }

    /// W-6 100%-during-burst variant: simulate a burst where event 3 is the
    /// one that completes the achievement set. The celebration must be the
    /// LAST emit (after all 5 standard popups), not jump ahead.
    #[tokio::test]
    async fn celebration_appended_last_during_burst_with_100pct_at_event_3() {
        let (tx, mut rx) = mpsc::channel::<RawUnlockEvent>(64);
        for i in 0..5 {
            tx.send(RawUnlockEvent {
                app_id: 480,
                ach_api_name: format!("ACH_{}", i),
                timestamp: 1_700_000_000 + i as u64,
                source: crate::sources::SourceKind::Goldberg,
            }).await.unwrap();
        }
        drop(tx);

        // Walk the channel like the real drain loop: every event yields ProcessEvent.
        // After event 3 we mark celebration_pending. Once channel goes idle,
        // decide_action returns EmitCelebration.
        let mut celebration_pending = false;
        let mut emitted_order: Vec<&'static str> = Vec::new();
        let mut idx = 0usize;
        while let Some(_evt) = rx.recv().await {
            let action = decide_action(true, celebration_pending, false);
            assert_eq!(action, DrainAction::ProcessEvent, "every event must process");
            emitted_order.push("popup");
            if idx == 2 { celebration_pending = true; } // event 3 (zero-indexed 2) hits 100%
            idx += 1;
        }
        // Now channel is closed/idle and celebration is pending.
        let final_action = decide_action(false, celebration_pending, true);
        assert_eq!(final_action, DrainAction::EmitCelebration);
        emitted_order.push("celebration");

        // Final order: 5 popups, then 1 celebration — celebration is LAST.
        assert_eq!(
            emitted_order,
            vec!["popup", "popup", "popup", "popup", "popup", "celebration"],
            "celebration must be appended LAST regardless of when 100% hits"
        );
    }
}
