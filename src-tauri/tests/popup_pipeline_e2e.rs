//! End-to-end test: synthesize a Goldberg unlock through the Phase 1 pipeline
//! and verify it arrives at the sink (which is where Plan 05's popup_queue
//! consumes from). Tauri webview emission is verified manually per ROADMAP
//! success criterion #1.
//!
//! What this test PROVES:
//!   • A RawUnlockEvent enters run_pipeline.
//!   • Cross-source dedup + record_unlock + sink-forward all succeed.
//!   • The kept event arrives at sink_rx within 1 second.
//!
//! What this test does NOT verify (manual smoke required):
//!   • Tauri popup-show event emission to the popup webview.
//!   • WS_EX_NOACTIVATE focus-steal behavior on a real game.
//!   • rodio audio output (no device on CI).
//!   • Burst-of-N drop-free behavior — covered by popup_queue's own unit tests
//!     (Plan 05 W-6 burst_of_5_events_produces_5_payloads_no_drops).

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

use hallmark_lib::sources::{RawUnlockEvent, SourceKind};
use hallmark_lib::store::{queries, SqliteStore};
use hallmark_lib::watcher::run_pipeline;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raw_unlock_event_arrives_at_sink_within_1s() {
    hallmark_lib::init_tracing_for_tests();

    let store = Arc::new(SqliteStore::open_in_memory().unwrap());
    let session_id = "test-session-1".to_string();
    store.with_conn(|c| queries::create_session(c, &session_id, None)).unwrap();

    let (raw_tx, raw_rx) = mpsc::channel::<RawUnlockEvent>(64);
    let (sink_tx, mut sink_rx) = mpsc::channel::<RawUnlockEvent>(64);

    // Spawn run_pipeline in the background.
    let pipeline = tokio::spawn(run_pipeline(
        raw_rx, store.clone(), session_id.clone(), sink_tx, Duration::from_secs(10),
    ));

    // Send a synthetic event.
    let evt = RawUnlockEvent {
        app_id: 480,
        ach_api_name: "ACH_TEST".to_string(),
        timestamp: 1_700_000_000,
        source: SourceKind::Goldberg,
    };
    raw_tx.send(evt.clone()).await.unwrap();

    // Verify the event arrives at the sink within 1 second (POPUP-01 latency target).
    let received = timeout(Duration::from_secs(1), sink_rx.recv()).await
        .expect("timeout waiting for sink event")
        .expect("sink closed unexpectedly");

    assert_eq!(received.app_id, 480);
    assert_eq!(received.ach_api_name, "ACH_TEST");
    assert_eq!(received.source, SourceKind::Goldberg);

    // Verify it's recorded in unlock_history.
    let count = store.with_conn(|c|
        queries::count_earned_for_app_session(c, 480, &session_id)
    ).unwrap();
    assert_eq!(count, 1, "unlock should be persisted");

    // Cleanup
    drop(raw_tx);
    let _ = timeout(Duration::from_millis(500), pipeline).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duplicate_unlocks_dedup_at_sink() {
    hallmark_lib::init_tracing_for_tests();

    let store = Arc::new(SqliteStore::open_in_memory().unwrap());
    let session_id = "test-session-2".to_string();
    store.with_conn(|c| queries::create_session(c, &session_id, None)).unwrap();

    let (raw_tx, raw_rx) = mpsc::channel::<RawUnlockEvent>(64);
    let (sink_tx, mut sink_rx) = mpsc::channel::<RawUnlockEvent>(64);

    let pipeline = tokio::spawn(run_pipeline(
        raw_rx, store.clone(), session_id.clone(), sink_tx, Duration::from_secs(10),
    ));

    let evt = RawUnlockEvent {
        app_id: 480, ach_api_name: "ACH_DEDUP".to_string(),
        timestamp: 0, source: SourceKind::Goldberg,
    };
    // Send three identical events.
    for _ in 0..3 { raw_tx.send(evt.clone()).await.unwrap(); }

    // Only one should arrive at sink.
    let _first = timeout(Duration::from_secs(1), sink_rx.recv()).await
        .expect("first").expect("sink closed");
    // No further within 200ms.
    let further = timeout(Duration::from_millis(200), sink_rx.recv()).await;
    assert!(further.is_err(), "duplicates must NOT reach sink");

    drop(raw_tx);
    let _ = timeout(Duration::from_millis(500), pipeline).await;
}
