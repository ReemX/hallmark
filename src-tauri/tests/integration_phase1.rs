//! Phase 1 end-to-end integration tests covering ROADMAP Success Criteria #1–#5.
//!
//! Each test runs the full pipeline (`run_watcher` + `run_pipeline`) against tempdir
//! fixtures. No `%APPDATA%` writes; no `cargo run --bin hallmark-cli` subprocesses
//! (we exercise the same library entry points the binary calls, but inline so we
//! can assert against the channels directly).
//!
//! Reference: .planning/ROADMAP.md → "Phase 1 Detection Pipeline Foundation → Success Criteria"

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hallmark_lib::paths::{self, DiscoveredPaths, GoldbergRedirect};
use hallmark_lib::sources::goldberg::GoldbergAdapter;
use hallmark_lib::sources::{RawUnlockEvent, SourceAdapter, SourceKind};
use hallmark_lib::store::SqliteStore;
use hallmark_lib::watcher::{run_pipeline, run_watcher};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context as LayerContext, SubscriberExt};
use tracing_subscriber::Layer;

// ============================================================================
// Common test helpers
// ============================================================================

fn fresh_tmp(label: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("hallmark-int-{}-{}", label, uuid::Uuid::new_v4()));
    fs::create_dir_all(&p).unwrap();
    p
}

fn write_state(root: &Path, app_id: u64, content: &str) -> PathBuf {
    let dir = root.join(app_id.to_string());
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("achievements.json");
    fs::write(&path, content).unwrap();
    path
}

fn write_appmanifest(library: &Path, app_id: u64, installdir: &str) {
    let steamapps = library.join("steamapps");
    fs::create_dir_all(&steamapps).unwrap();
    let content = format!(
        "\"AppState\"\n{{\n  \"appid\"      \"{}\"\n  \"name\"       \"Test\"\n  \"installdir\" \"{}\"\n}}\n",
        app_id, installdir
    );
    fs::write(
        steamapps.join(format!("appmanifest_{}.acf", app_id)),
        content,
    )
    .unwrap();
}

/// Build the full Phase 1 pipeline (run_watcher + run_pipeline) returning the
/// sink receiver + the store handle for assertions.
async fn spawn_pipeline(
    adapters: Vec<Arc<dyn SourceAdapter>>,
    store: Arc<SqliteStore>,
) -> (
    mpsc::Receiver<RawUnlockEvent>,
    tokio::task::JoinHandle<anyhow::Result<()>>,
    tokio::task::JoinHandle<anyhow::Result<()>>,
) {
    let (raw_tx, raw_rx) = mpsc::channel::<RawUnlockEvent>(64);
    let (sink_tx, sink_rx) = mpsc::channel::<RawUnlockEvent>(64);
    let watcher_handle = tokio::spawn(run_watcher(adapters, raw_tx));
    let pipeline_handle = tokio::spawn(run_pipeline(
        raw_rx,
        store,
        "test-session".to_string(),
        sink_tx,
        Duration::from_secs(10),
    ));
    // Allow seeding + watcher attach.
    tokio::time::sleep(Duration::from_millis(400)).await;
    (sink_rx, watcher_handle, pipeline_handle)
}

// ============================================================================
// SC1: single-event detection within 1.5s
// ============================================================================

#[tokio::test]
async fn sc1_single_unlock_emits_exactly_one_event_within_one_second() {
    let root = fresh_tmp("sc1");
    let baseline = r#"{
            "ACH_X": {"earned": false, "earned_time": 0},
            "ACH_Y": {"earned": false, "earned_time": 0}
        }"#;
    let path = write_state(&root, 480, baseline);

    let store = Arc::new(SqliteStore::open_in_memory().unwrap());
    let adapter: Arc<dyn SourceAdapter> =
        Arc::new(GoldbergAdapter::new(vec![root.clone()], HashMap::new()));
    let (mut sink_rx, watch, pipe) = spawn_pipeline(vec![adapter], store.clone()).await;

    // Mark ACH_X earned
    fs::write(
        &path,
        r#"{
            "ACH_X": {"earned": true, "earned_time": 1700000999},
            "ACH_Y": {"earned": false, "earned_time": 0}
        }"#,
    )
    .unwrap();

    let evt = timeout(Duration::from_millis(1500), sink_rx.recv())
        .await
        .expect("event should arrive within 1.5s")
        .expect("Some(event)");
    assert_eq!(evt.app_id, 480);
    assert_eq!(evt.ach_api_name, "ACH_X");

    // No duplicates for the next 2 seconds.
    let none = timeout(Duration::from_secs(2), sink_rx.recv()).await;
    assert!(
        none.is_err() || none.unwrap().is_none(),
        "no duplicate events within 2s window (Success Criterion #1)"
    );

    assert_eq!(store.count_unlocks().unwrap(), 1);

    watch.abort();
    pipe.abort();
    let _ = fs::remove_dir_all(&root);
}

// ============================================================================
// SC2: pre-populated state emits zero events
// ============================================================================

#[tokio::test]
async fn sc2_pre_populated_state_emits_zero_events() {
    let root = fresh_tmp("sc2");
    let mut entries = Vec::with_capacity(50);
    for i in 0..50 {
        entries.push(format!(
            r#""ACH_{:03}":{{"earned":true,"earned_time":{}}}"#,
            i,
            1_700_000_000 + i
        ));
    }
    let baseline = format!("{{ {} }}", entries.join(","));
    write_state(&root, 480, &baseline);

    let store = Arc::new(SqliteStore::open_in_memory().unwrap());
    let adapter: Arc<dyn SourceAdapter> =
        Arc::new(GoldbergAdapter::new(vec![root.clone()], HashMap::new()));
    let (mut sink_rx, watch, pipe) = spawn_pipeline(vec![adapter], store.clone()).await;

    // Wait 1500ms — well past the debounce window. No events should arrive.
    let none = timeout(Duration::from_millis(1500), sink_rx.recv()).await;
    assert!(
        none.is_err() || none.unwrap().is_none(),
        "zero historical unlock events (Success Criterion #2)"
    );

    assert_eq!(store.count_unlocks().unwrap(), 0);

    watch.abort();
    pipe.abort();
    let _ = fs::remove_dir_all(&root);
}

// ============================================================================
// SC3: real-disk local_save.txt redirect pipeline (B-01 fix)
//
// Build a complete Steam-library-shaped fixture, run real path discovery,
// construct a real GoldbergAdapter from the discovered redirect_map, run the
// full pipeline, and write achievements.json to the resolved redirect target.
// Assert exactly one event arrives with the appid resolved from appmanifest.
// ============================================================================

#[tokio::test]
async fn sc3_local_save_txt_redirect_drives_end_to_end_pipeline() {
    // ---- Build Steam-library-shaped fixture on disk ----
    let lib = fresh_tmp("sc3-lib");
    let common = lib.join("steamapps").join("common");
    let game_bin = common.join("FooGame").join("bin");
    fs::create_dir_all(&game_bin).unwrap();
    fs::write(game_bin.join("steam_api64.dll"), b"placeholder").unwrap();

    // appmanifest mapping installdir "FooGame" → appid 4242
    write_appmanifest(&lib, 4242, "FooGame");

    // local_save.txt redirect target. BL-04: must be under the DLL's directory
    // tree (or under a known Goldberg default root) to satisfy the path-traversal
    // allow-list. Place it as a sibling subdir of the DLL dir.
    let redirect_target = game_bin.join("sc3-redirect");
    fs::create_dir_all(&redirect_target).unwrap();
    let target_str = redirect_target.to_string_lossy().replace('/', "\\");
    fs::write(game_bin.join("local_save.txt"), &target_str).unwrap();

    // ---- Drive real path discovery ----
    let redirects: Vec<GoldbergRedirect> =
        paths::scan_local_save_redirects_pub_for_tests(&[lib.clone()]);
    assert_eq!(
        redirects.len(),
        1,
        "expected one redirect from fixture; got {:?}",
        redirects
    );
    assert_eq!(redirects[0].target_path, redirect_target);
    assert_eq!(redirects[0].app_id, 4242);

    // Build the redirect_map the same way the CLI binary does.
    let discovered = DiscoveredPaths {
        steam_install: Some(lib.clone()),
        steam_libraries: vec![lib.clone()],
        goldberg_save_roots: vec![],
        goldberg_local_save_redirects: redirects.clone(),
    };
    let redirect_map = paths::goldberg_redirect_map(&discovered);
    assert_eq!(
        redirect_map.get(&redirect_target).copied(),
        Some(4242),
        "redirect_map should pair target_path → appid"
    );

    // Seed the redirect target with a baseline state file BEFORE spawning the pipeline,
    // so the adapter sees `ACH_X: false` at seed time and the later `false→true` flip
    // is the only transition.
    let ach_path = redirect_target.join("achievements.json");
    fs::write(&ach_path, r#"{"ACH_X":{"earned":false,"earned_time":0}}"#).unwrap();

    // ---- Build adapter + run pipeline ----
    let store = Arc::new(SqliteStore::open_in_memory().unwrap());
    let adapter: Arc<dyn SourceAdapter> = Arc::new(GoldbergAdapter::new(vec![], redirect_map));
    let (mut sink_rx, watch, pipe) = spawn_pipeline(vec![adapter], store.clone()).await;

    // ---- Trigger the unlock by mutating achievements.json under the redirect target ----
    fs::write(
        &ach_path,
        r#"{"ACH_X":{"earned":true,"earned_time":1700001234}}"#,
    )
    .unwrap();

    let evt = timeout(Duration::from_millis(1500), sink_rx.recv())
        .await
        .expect("event should arrive within 1.5s")
        .expect("Some(event)");
    assert_eq!(
        evt.app_id, 4242,
        "appid should be resolved from appmanifest (4242), NOT the redirect target's parent dir name"
    );
    assert_eq!(evt.ach_api_name, "ACH_X");
    assert_eq!(evt.source, SourceKind::Goldberg);

    let none = timeout(Duration::from_millis(800), sink_rx.recv()).await;
    assert!(
        none.is_err() || none.unwrap().is_none(),
        "no duplicate events"
    );
    assert_eq!(store.count_unlocks().unwrap(), 1);

    watch.abort();
    pipe.abort();
    let _ = fs::remove_dir_all(&lib);
    let _ = fs::remove_dir_all(&redirect_target);
}

// ============================================================================
// SC4: cross-source dedup using two real MockAdapter instances (W-08 fix)
//
// Each MockAdapter watches its own tempdir root and emits `RawUnlockEvent` when
// its `<root>/trigger.json` file's `earned` value flips from false to true.
// We trigger BOTH adapters' files near-simultaneously; the dedup stage drops
// the second event.
// ============================================================================

/// Test-only adapter that mirrors GoldbergAdapter's contract minimally:
/// watches a single root, on `trigger.json` change reads "{earned: bool}" and
/// emits a single `RawUnlockEvent` for `(fixed_app_id, fixed_ach)` on `false→true`.
struct MockAdapter {
    root: PathBuf,
    fixed_app_id: u64,
    fixed_ach: String,
    baseline: Arc<tokio::sync::RwLock<Option<bool>>>,
}

impl MockAdapter {
    fn new(root: PathBuf, fixed_app_id: u64, fixed_ach: &str) -> Self {
        Self {
            root,
            fixed_app_id,
            fixed_ach: fixed_ach.to_string(),
            baseline: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl SourceAdapter for MockAdapter {
    fn name(&self) -> &str {
        "mock"
    }
    fn kind(&self) -> SourceKind {
        SourceKind::Goldberg
    }
    fn watch_paths(&self) -> Vec<PathBuf> {
        if self.root.exists() {
            vec![self.root.clone()]
        } else {
            vec![]
        }
    }
    async fn seed_baseline(&self) -> anyhow::Result<()> {
        let trigger = self.root.join("trigger.json");
        let val = if trigger.exists() {
            let s = fs::read_to_string(&trigger).unwrap_or_default();
            serde_json::from_str::<serde_json::Value>(&s)
                .ok()
                .and_then(|v| v.get("earned").and_then(|e| e.as_bool()))
                .unwrap_or(false)
        } else {
            false
        };
        *self.baseline.write().await = Some(val);
        Ok(())
    }
    async fn on_file_changed(
        &self,
        path: PathBuf,
        tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()> {
        if path.file_name().and_then(|n| n.to_str()) != Some("trigger.json") {
            return Ok(());
        }
        let s = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Ok(()),
        };
        let earned_now = serde_json::from_str::<serde_json::Value>(&s)
            .ok()
            .and_then(|v| v.get("earned").and_then(|e| e.as_bool()))
            .unwrap_or(false);
        // WR-10: hold the write lock across the read-emit-update sequence so two
        // concurrent invocations cannot both observe `was = false` and both emit.
        // The previous read-then-write split allowed a TOCTOU window across the
        // tx.send().await suspension point. Downstream dedup catches the duplicate
        // anyway, but the test should reflect the contract GoldbergAdapter
        // upholds rather than relying on dedup as a safety net.
        let mut baseline = self.baseline.write().await;
        let was = baseline.unwrap_or(false);
        if !was && earned_now {
            let _ = tx
                .send(RawUnlockEvent {
                    app_id: self.fixed_app_id,
                    ach_api_name: self.fixed_ach.clone(),
                    timestamp: 0,
                    source: SourceKind::Goldberg,
                })
                .await;
        }
        *baseline = Some(earned_now);
        Ok(())
    }
}

#[tokio::test]
async fn sc4_cross_source_dedup_collapses_real_adapter_events_to_one() {
    let root_a = fresh_tmp("sc4-a");
    let root_b = fresh_tmp("sc4-b");
    // Both adapters start with `earned: false` so the seed picks up false; the later
    // write of `earned: true` is a true transition for each.
    fs::write(root_a.join("trigger.json"), r#"{"earned":false}"#).unwrap();
    fs::write(root_b.join("trigger.json"), r#"{"earned":false}"#).unwrap();

    let mock_a: Arc<dyn SourceAdapter> =
        Arc::new(MockAdapter::new(root_a.clone(), 4242, "ACH_DUP"));
    let mock_b: Arc<dyn SourceAdapter> =
        Arc::new(MockAdapter::new(root_b.clone(), 4242, "ACH_DUP"));

    let store = Arc::new(SqliteStore::open_in_memory().unwrap());
    let (mut sink_rx, watch, pipe) = spawn_pipeline(vec![mock_a, mock_b], store.clone()).await;

    // Flip both files near-simultaneously
    fs::write(root_a.join("trigger.json"), r#"{"earned":true}"#).unwrap();
    fs::write(root_b.join("trigger.json"), r#"{"earned":true}"#).unwrap();

    let first = timeout(Duration::from_millis(2000), sink_rx.recv())
        .await
        .expect("first event")
        .expect("Some");
    assert_eq!(first.app_id, 4242);
    assert_eq!(first.ach_api_name, "ACH_DUP");

    // Second event must be dropped within the dedup TTL window.
    let none = timeout(Duration::from_millis(800), sink_rx.recv()).await;
    assert!(
        none.is_err() || none.unwrap().is_none(),
        "second event must be dropped at dedup stage (Success Criterion #4)"
    );

    assert_eq!(
        store.count_unlocks().unwrap(),
        1,
        "exactly one row persisted (Success Criterion #4)"
    );

    watch.abort();
    pipe.abort();
    let _ = fs::remove_dir_all(&root_a);
    let _ = fs::remove_dir_all(&root_b);
}

// ============================================================================
// SC5: tracing capture proves every discovery category emits an info event
// ============================================================================

struct VecLayer {
    events: Arc<Mutex<Vec<String>>>,
}
impl<S: Subscriber> Layer<S> for VecLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: LayerContext<'_, S>) {
        use tracing::field::{Field, Visit};
        struct V(String);
        impl Visit for V {
            fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                self.0.push_str(&format!(" {}={:?}", f.name(), v));
            }
            fn record_str(&mut self, f: &Field, v: &str) {
                self.0.push_str(&format!(" {}={}", f.name(), v));
            }
        }
        let mut v = V(String::new());
        event.record(&mut v);
        self.events
            .lock()
            .unwrap()
            .push(format!("{} :: {}", event.metadata().level(), v.0));
    }
}

#[test]
fn sc5_path_discovery_logs_every_category_to_tracing() {
    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let layer = VecLayer {
        events: events.clone(),
    };
    let subscriber = tracing_subscriber::registry().with(layer);
    let _guard = tracing::subscriber::set_default(subscriber);

    let d = DiscoveredPaths {
        steam_install: Some(PathBuf::from(r"C:\FakeSteam")),
        steam_libraries: vec![
            PathBuf::from(r"C:\FakeSteam"),
            PathBuf::from(r"D:\FakeLibrary"),
        ],
        goldberg_save_roots: vec![PathBuf::from(r"C:\Goldberg")],
        goldberg_local_save_redirects: vec![GoldbergRedirect {
            target_path: PathBuf::from(r"D:\Redirect"),
            app_id: 4242,
        }],
    };
    paths::log_discovery_pub_for_tests(&d);

    let captured = events.lock().unwrap().clone();
    assert!(
        captured.iter().any(|e| e.contains("Steam install")),
        "expected 'Steam install' info event; got: {:?}",
        captured
    );
    assert!(
        captured.iter().any(|e| e.contains("Steam library")),
        "expected 'Steam library' info event; got: {:?}",
        captured
    );
    assert!(
        captured.iter().any(|e| e.contains("Goldberg save root")),
        "expected 'Goldberg save root' info event; got: {:?}",
        captured
    );
    assert!(
        captured
            .iter()
            .any(|e| e.contains("local_save.txt redirect")),
        "expected 'local_save.txt redirect' info event; got: {:?}",
        captured
    );
    let info_count = captured.iter().filter(|e| e.starts_with("INFO")).count();
    assert!(
        info_count >= 4,
        "expected at least 4 INFO-level events; got: {:?}",
        captured
    );
}
