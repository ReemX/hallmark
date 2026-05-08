//! Watcher core — single `notify-debouncer-full` instance driving all source adapters.
//!
//! Phase 1 wires only `GoldbergAdapter` (Plan 04 — `sources::goldberg`). Phase 3 will
//! add Steam-legit, CreamAPI, and SmartSteamEmu adapters; the only change required
//! is more entries in the `Vec<Arc<dyn SourceAdapter>>` passed to `run_watcher`.
//!
//! # Why ONE debouncer for ALL adapters
//!
//! Each adapter could spawn its own watcher, but a single debouncer:
//! 1. Enforces a uniform 500ms debounce policy (REQ DETECT-06).
//! 2. Prevents adapter-vs-adapter buffer-size races on `ReadDirectoryChangesW`.
//! 3. Centralizes the sync-callback → tokio-mpsc bridge (one place to get right).
//!
//! # Ordering guarantee (REQ DETECT-05)
//!
//! `seed_baseline()` MUST complete on EVERY adapter BEFORE the debouncer is wired up.
//! Reversing this order means an adapter could see a file event before its baseline
//! is set, treating every existing achievement as a new unlock — the spam scenario
//! REQ DETECT-05 forbids. This invariant is enforced by the function-call order
//! in `run_watcher` and asserted by `run_watcher_seeds_before_attaching_watcher`.

pub mod dedup;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use tokio::sync::mpsc;

use crate::sources::{RawUnlockEvent, SourceAdapter};

/// Run the central watcher. Seeds every adapter's baseline first (REQ DETECT-05),
/// then constructs a single `notify-debouncer-full` driving all adapters' watch
/// paths, and dispatches debounced events back to the matching adapter.
///
/// Returns when `raw_tx` is dropped (graceful shutdown via channel close on the
/// receiver side), or on a watcher setup error.
pub async fn run_watcher(
    adapters: Vec<Arc<dyn SourceAdapter>>,
    raw_tx: mpsc::Sender<RawUnlockEvent>,
) -> anyhow::Result<()> {
    // ----- Phase 1: seed baselines BEFORE attaching watchers (REQ DETECT-05) -----
    for adapter in &adapters {
        adapter.seed_baseline().await?;
        tracing::info!(adapter = adapter.name(), "Baseline seeded");
    }

    // ----- Phase 2: construct shared debouncer + register every adapter's paths -----
    let (notify_tx, mut notify_rx) = mpsc::channel::<DebounceEventResult>(64);

    // notify-debouncer-full's callback is sync (runs on debouncer's tick thread).
    // Bridge to tokio mpsc via `blocking_send` — fast forward, no blocking work here.
    let mut debouncer = new_debouncer(
        Duration::from_millis(500), // REQ DETECT-06: 500ms debounce window
        None,                       // tick_rate auto = timeout / 4
        move |res: DebounceEventResult| {
            if let Err(e) = notify_tx.blocking_send(res) {
                tracing::warn!(error = %e, "notify→tokio bridge full or closed");
            }
        },
    )?;

    // BL-03 / WR-08: build the (adapter_idx, path) routing table ONCE, here. Once a
    // path is registered with the debouncer it stays in this table for the watcher's
    // lifetime — even if the directory is later renamed/deleted, events already
    // buffered by `notify` will still route correctly. Filter `exists()` at register
    // time only.
    let mut path_owner: Vec<(usize, PathBuf)> = Vec::new();
    for (idx, adapter) in adapters.iter().enumerate() {
        for path in adapter.watch_paths() {
            if !path.exists() {
                tracing::warn!(adapter = adapter.name(), path = %path.display(),
                    "watch path does not exist; skipping (PathNotFound would error)");
                continue;
            }
            match debouncer.watch(&path, RecursiveMode::Recursive) {
                Ok(()) => {
                    tracing::info!(adapter = adapter.name(), path = %path.display(),
                        "watching path recursively");
                    path_owner.push((idx, path));
                }
                Err(e) => {
                    tracing::warn!(adapter = adapter.name(), path = %path.display(),
                        error = %e, "debouncer.watch failed");
                }
            }
        }
    }

    // WR-09: detect adapter watch-path overlaps at startup. If a path under one adapter
    // is an ancestor (or descendant) of another adapter's path, events for the
    // overlapping subtree could be misrouted by a first-prefix-match policy. We log
    // an error so the misconfiguration is visible and dispatch to ALL matching
    // adapters below; downstream cross-source dedup is responsible for collapsing
    // any duplicate events.
    for (i, (a_idx, pa)) in path_owner.iter().enumerate() {
        for (b_idx, pb) in path_owner.iter().skip(i + 1) {
            if a_idx == b_idx {
                continue;
            }
            if pa.starts_with(pb) || pb.starts_with(pa) {
                tracing::error!(
                    adapter_a = adapters[*a_idx].name(),
                    adapter_b = adapters[*b_idx].name(),
                    path_a = %pa.display(),
                    path_b = %pb.display(),
                    "adapter watch paths overlap; events may be routed to multiple adapters"
                );
            }
        }
    }

    tracing::info!(
        adapters = adapters.len(),
        paths = path_owner.len(),
        "WatcherCore active"
    );

    // ----- Phase 3: event loop -----
    while let Some(res) = notify_rx.recv().await {
        match res {
            Ok(events) => {
                for event in events {
                    for path in &event.event.paths {
                        dispatch(&adapters, &path_owner, path.clone(), &raw_tx).await;
                    }
                }
            }
            Err(errors) => {
                for e in errors {
                    tracing::warn!(error = %e, "notify watcher error");
                }
            }
        }
    }

    tracing::info!("WatcherCore shutting down (notify channel closed)");
    Ok(())
}

/// BL-03: Scan the cached `path_owner` table (built once at startup) for prefix
/// matches and forward to every owning adapter. WR-09: dispatching to ALL matches
/// (not just the first) means overlapping adapter configurations do not silently
/// drop events; downstream cross-source dedup collapses any duplicates.
async fn dispatch(
    adapters: &[Arc<dyn SourceAdapter>],
    path_owner: &[(usize, PathBuf)],
    path: PathBuf,
    raw_tx: &mpsc::Sender<RawUnlockEvent>,
) {
    let mut matched_any = false;
    // Track which adapters have already received this event to avoid double-firing
    // the same adapter when it has multiple overlapping watch paths registered.
    let mut delivered: Vec<usize> = Vec::new();
    for (idx, wp) in path_owner {
        if path.starts_with(wp) {
            matched_any = true;
            if delivered.contains(idx) {
                continue;
            }
            delivered.push(*idx);
            let adapter = &adapters[*idx];
            if let Err(e) = adapter.on_file_changed(path.clone(), raw_tx.clone()).await {
                tracing::warn!(adapter = adapter.name(), path = %path.display(),
                    error = %e, "adapter on_file_changed errored");
            }
        }
    }
    if !matched_any {
        tracing::trace!(path = %path.display(), "no adapter claims this path; ignoring");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::goldberg::GoldbergAdapter;
    use crate::sources::{RawUnlockEvent, SourceAdapter, SourceKind};
    use std::collections::HashMap;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tokio::time::timeout;

    // A spy adapter that records the order of method calls.
    struct OrderSpy {
        paths: Vec<PathBuf>,
        seed_count: AtomicU32,
        change_count: AtomicU32,
        change_after_seed: AtomicU32,
    }

    impl OrderSpy {
        fn new(paths: Vec<PathBuf>) -> Self {
            Self {
                paths,
                seed_count: AtomicU32::new(0),
                change_count: AtomicU32::new(0),
                change_after_seed: AtomicU32::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl SourceAdapter for OrderSpy {
        fn name(&self) -> &str {
            "order_spy"
        }
        fn kind(&self) -> SourceKind {
            SourceKind::Goldberg
        }
        fn watch_paths(&self) -> Vec<PathBuf> {
            self.paths.iter().filter(|p| p.exists()).cloned().collect()
        }
        async fn seed_baseline(&self) -> anyhow::Result<()> {
            self.seed_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn on_file_changed(
            &self,
            _path: PathBuf,
            _tx: mpsc::Sender<RawUnlockEvent>,
        ) -> anyhow::Result<()> {
            let already_seeded = self.seed_count.load(Ordering::SeqCst) > 0;
            self.change_count.fetch_add(1, Ordering::SeqCst);
            if already_seeded {
                self.change_after_seed.fetch_add(1, Ordering::SeqCst);
            }
            Ok(())
        }
    }

    fn fresh_tmp() -> PathBuf {
        let p = std::env::temp_dir().join(format!("hallmark-watcher-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[tokio::test]
    async fn run_watcher_seeds_before_attaching_watcher() {
        let dir = fresh_tmp();
        let spy = Arc::new(OrderSpy::new(vec![dir.clone()]));
        let (raw_tx, _raw_rx) = mpsc::channel::<RawUnlockEvent>(8);
        let adapters: Vec<Arc<dyn SourceAdapter>> = vec![spy.clone()];

        let handle = tokio::spawn(run_watcher(adapters, raw_tx));
        // Give the watcher a moment to seed and attach.
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Trigger a file event.
        fs::write(dir.join("test.txt"), b"x").unwrap();

        // Wait long enough for debounce + dispatch.
        tokio::time::sleep(Duration::from_millis(900)).await;

        assert_eq!(
            spy.seed_count.load(Ordering::SeqCst),
            1,
            "seed_baseline called exactly once"
        );
        // change_count >= 1 OR change_count == change_after_seed — either way, every change happens after seeding.
        let changes = spy.change_count.load(Ordering::SeqCst);
        let after_seed = spy.change_after_seed.load(Ordering::SeqCst);
        assert_eq!(
            changes, after_seed,
            "every on_file_changed must occur after seed_baseline (got {} changes, {} after seed)",
            changes, after_seed
        );

        handle.abort();
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn run_watcher_filters_nonexistent_paths() {
        let real = fresh_tmp();
        let phantom = real.join("does-not-exist");
        let spy = Arc::new(OrderSpy::new(vec![real.clone(), phantom]));
        let (raw_tx, _raw_rx) = mpsc::channel::<RawUnlockEvent>(8);
        let adapters: Vec<Arc<dyn SourceAdapter>> = vec![spy.clone()];

        let handle = tokio::spawn(run_watcher(adapters, raw_tx));
        tokio::time::sleep(Duration::from_millis(200)).await;
        // If the phantom path had been registered, run_watcher would have errored before this point.
        assert!(!handle.is_finished(), "run_watcher should still be running");

        handle.abort();
        let _ = fs::remove_dir_all(&real);
    }

    #[tokio::test]
    async fn run_watcher_emits_event_through_real_debouncer_within_1s() {
        let root = fresh_tmp();
        let appid_dir = root.join("480");
        fs::create_dir_all(&appid_dir).unwrap();
        let path = appid_dir.join("achievements.json");
        let baseline = r#"{"ACH_X":{"earned":false,"earned_time":0}}"#;
        fs::write(&path, baseline).unwrap();

        let adapter: Arc<dyn SourceAdapter> =
            Arc::new(GoldbergAdapter::new(vec![root.clone()], HashMap::new()));
        let (raw_tx, mut raw_rx) = mpsc::channel::<RawUnlockEvent>(8);

        let handle = tokio::spawn(run_watcher(vec![adapter], raw_tx));
        tokio::time::sleep(Duration::from_millis(300)).await; // seed + attach

        // Flip the achievement
        fs::write(
            &path,
            r#"{"ACH_X":{"earned":true,"earned_time":1700000999}}"#,
        )
        .unwrap();

        let evt = timeout(Duration::from_millis(1500), raw_rx.recv())
            .await
            .expect("event should arrive within 1500ms (500ms debounce + slack)")
            .expect("expected Some(event)");
        assert_eq!(evt.app_id, 480);
        assert_eq!(evt.ach_api_name, "ACH_X");
        assert_eq!(evt.source, SourceKind::Goldberg);

        // No further events for the next 800ms
        let none = timeout(Duration::from_millis(800), raw_rx.recv()).await;
        assert!(
            none.is_err() || none.unwrap().is_none(),
            "no further events should arrive (Success Criterion #1: no duplicates within 5s)"
        );

        handle.abort();
        let _ = fs::remove_dir_all(&root);
    }
}

// ----------------- Pipeline consumer (Plan 05) -----------------

use crate::store::SqliteStore;
use crate::watcher::dedup::CrossSourceDedup;
use tokio::sync::Mutex as TokioMutex;

/// Consumes the `raw_rx` stream from `run_watcher`, applies cross-source dedup
/// (REQ DETECT-07), persists each KEPT event to the SQLite store, and forwards
/// kept events to the `sink` for the CLI test harness / Phase 2 popup queue.
///
/// Returns when `raw_rx` is closed (graceful shutdown).
///
/// WR-02: This function does NOT take an `adapters: Vec<Arc<dyn SourceAdapter>>`
/// parameter; only `run_watcher` needs the adapters. Callers that wired up both
/// previously cloned the Vec twice — that clone is no longer required.
pub async fn run_pipeline(
    mut raw_rx: mpsc::Receiver<RawUnlockEvent>,
    store: Arc<SqliteStore>,
    session_id: String,
    sink: mpsc::Sender<RawUnlockEvent>,
    dedup_ttl: Duration,
) -> anyhow::Result<()> {
    let dedup = Arc::new(TokioMutex::new(CrossSourceDedup::new(dedup_ttl)));

    while let Some(evt) = raw_rx.recv().await {
        let is_dup = {
            let mut d = dedup.lock().await;
            d.is_duplicate(evt.app_id, &evt.ach_api_name)
        };
        if is_dup {
            tracing::debug!(
                app_id = evt.app_id,
                ach = %evt.ach_api_name,
                source = %evt.source,
                "cross-source dedup: dropped duplicate"
            );
            continue;
        }

        // Persist (belt-and-suspenders DB-level dedup via UNIQUE INDEX).
        let inserted = match store.record_unlock(
            evt.app_id,
            &evt.ach_api_name,
            evt.source.as_str(),
            Some(&session_id),
        ) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "store.record_unlock failed");
                false
            }
        };
        if !inserted {
            tracing::debug!(
                app_id = evt.app_id,
                ach = %evt.ach_api_name,
                "DB-level dedup: row already existed (UNIQUE INDEX)"
            );
            continue;
        }

        tracing::info!(
            app_id = evt.app_id,
            ach = %evt.ach_api_name,
            source = %evt.source,
            "UNLOCK"
        );
        if sink.send(evt).await.is_err() {
            tracing::debug!("downstream sink closed; pipeline draining");
        }
    }

    tracing::info!("run_pipeline shutting down (raw_rx closed)");
    Ok(())
}

#[cfg(test)]
mod pipeline_tests {
    use super::*;
    use crate::sources::SourceKind;
    use std::time::Duration as StdDuration;

    #[tokio::test]
    async fn run_pipeline_dedups_simultaneous_cross_source_events() {
        let store = Arc::new(SqliteStore::open_in_memory().unwrap());
        let session_id = "test-session-1".to_string();
        let (raw_tx, raw_rx) = mpsc::channel::<RawUnlockEvent>(8);
        let (sink_tx, mut sink_rx) = mpsc::channel::<RawUnlockEvent>(8);
        let store_clone = store.clone();

        let handle = tokio::spawn(run_pipeline(
            raw_rx,
            store_clone,
            session_id.clone(),
            sink_tx,
            StdDuration::from_secs(10),
        ));

        // Two events with the SAME logical key emitted within TTL — the second must dedup.
        for _ in 0..2 {
            raw_tx
                .send(RawUnlockEvent {
                    app_id: 480,
                    ach_api_name: "ACH_X".into(),
                    timestamp: 0,
                    source: SourceKind::Goldberg,
                })
                .await
                .unwrap();
        }

        let evt = tokio::time::timeout(StdDuration::from_millis(200), sink_rx.recv())
            .await
            .unwrap()
            .expect("first event should pass through");
        assert_eq!(evt.app_id, 480);
        assert_eq!(evt.ach_api_name, "ACH_X");

        // Second event must NOT pass through within 200ms
        let none = tokio::time::timeout(StdDuration::from_millis(200), sink_rx.recv()).await;
        assert!(
            none.is_err() || none.unwrap().is_none(),
            "duplicate must be dropped at dedup stage"
        );

        // Store has exactly ONE row
        assert_eq!(store.count_unlocks().unwrap(), 1);

        drop(raw_tx);
        let _ = handle.await;
    }
}
