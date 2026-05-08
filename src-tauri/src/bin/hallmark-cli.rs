//! `hallmark-cli` — Phase 1 CLI test harness.
//!
//! Wires the detection pipeline end-to-end without launching a Tauri WebView:
//!
//!     paths::discover()  →  GoldbergAdapter::new(...)  →  run_watcher
//!                                                              │
//!                                                              ▼
//!                                           run_pipeline (dedup + SQLite store)
//!                                                              │
//!                                                              ▼
//!                                                      stdout println per kept event
//!
//! # Usage
//!
//! Default — uses real `%APPDATA%` paths discovered by `paths::discover()`:
//!     cargo run --bin hallmark-cli
//!
//! Override — for integration tests / fixtures:
//!     cargo run --bin hallmark-cli -- --override-goldberg-root C:\path\to\fixture
//! Or env var (preferred for tests; argv requires the `--` separator):
//!     HALLMARK_GOLDBERG_ROOT_OVERRIDE=C:\path\to\fixture cargo run --bin hallmark-cli
//!
//! Exit: Ctrl-C, or close the input via piping.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use hallmark_lib::paths;
use hallmark_lib::sources::{goldberg::GoldbergAdapter, RawUnlockEvent, SourceAdapter};
use hallmark_lib::store::{queries, SqliteStore};
use hallmark_lib::watcher::{run_pipeline, run_watcher};
use tokio::sync::mpsc;
use uuid::Uuid;

fn parse_argv_override() -> Option<PathBuf> {
    // Accept either: `--override-goldberg-root <PATH>` argv, or env var.
    if let Ok(env_val) = std::env::var("HALLMARK_GOLDBERG_ROOT_OVERRIDE") {
        if !env_val.is_empty() {
            return Some(PathBuf::from(env_val));
        }
    }
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--override-goldberg-root" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

fn db_path() -> PathBuf {
    // Allow tests to override the DB path too; defaults to in-process tempdir for
    // Phase 1 so the CLI doesn't pollute %APPDATA%\Hallmark\ during unit work.
    if let Ok(p) = std::env::var("HALLMARK_DB_PATH_OVERRIDE") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    // Default: %APPDATA%\Hallmark\hallmark.db
    if let Some(appdata) = dirs::data_dir() {
        let dir = appdata.join("Hallmark");
        if std::fs::create_dir_all(&dir).is_ok() {
            return dir.join("hallmark.db");
        }
    }
    // Fallback: temp dir
    std::env::temp_dir().join("hallmark.db")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hallmark_lib::init_tracing();
    tracing::info!("hallmark-cli starting (Phase 1 detection-only harness)");

    // ---- Resolve watch paths + redirect_map ----
    let (goldberg_roots, redirect_map): (Vec<PathBuf>, HashMap<PathBuf, u64>) =
        if let Some(override_path) = parse_argv_override() {
            tracing::info!(path = %override_path.display(),
                "using --override-goldberg-root (real path discovery skipped)");
            (vec![override_path], HashMap::new())
        } else {
            let discovered = paths::discover();
            let roots = paths::goldberg_watch_paths(&discovered);
            let map = paths::goldberg_redirect_map(&discovered);
            (roots, map)
        };

    if goldberg_roots.is_empty() && redirect_map.is_empty() {
        tracing::warn!("no Goldberg paths discovered; pipeline will be idle");
    }

    // ---- Build adapters ----
    let adapter: Arc<dyn SourceAdapter> =
        Arc::new(GoldbergAdapter::new(goldberg_roots, redirect_map));
    let adapters = vec![adapter];

    // ---- Open store + create session ----
    let store = Arc::new(SqliteStore::open(&db_path())?);
    let session_id = Uuid::new_v4().to_string();
    store.with_conn(|conn| queries::create_session(conn, &session_id, None))?;
    tracing::info!(session_id = %session_id, "session created");

    // ---- Wire channels: watcher ──[raw_*]→ pipeline ──[sink_*]→ stdout printer ----
    let (raw_tx, raw_rx) = mpsc::channel::<RawUnlockEvent>(64);
    let (sink_tx, mut sink_rx) = mpsc::channel::<RawUnlockEvent>(64);

    let watcher_handle = tokio::spawn(run_watcher(adapters, raw_tx));
    let pipeline_handle = tokio::spawn(run_pipeline(
        raw_rx,
        store.clone(),
        session_id.clone(),
        sink_tx,
        Duration::from_secs(10),
    ));

    // ---- Stdout printer (the user-visible deliverable for ROADMAP Criterion #1) ----
    let printer_handle = tokio::spawn(async move {
        while let Some(evt) = sink_rx.recv().await {
            println!(
                "UNLOCK app_id={} ach={} source={}",
                evt.app_id, evt.ach_api_name, evt.source
            );
        }
    });

    // ---- Shutdown signal ----
    // Block on Ctrl-C; on signal, propagate to subtasks via channel-drop.
    tokio::signal::ctrl_c().await.ok();
    tracing::info!("Ctrl-C received; shutting down");

    // End the session in the DB.
    let _ = store.with_conn(|conn| queries::end_session(conn, &session_id));

    // Aborting watcher closes raw_tx → run_pipeline's recv() returns None → it exits → sink_tx drops → printer exits.
    watcher_handle.abort();
    let _ = pipeline_handle.await;
    let _ = printer_handle.await;

    tracing::info!("hallmark-cli stopped cleanly");
    Ok(())
}
