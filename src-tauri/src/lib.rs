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

use tracing_subscriber::EnvFilter;

/// Initialize structured logging. Call once at process start.
/// Reads RUST_LOG env var; defaults to `hallmark_lib=info,warn` for clean output.
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("hallmark_lib=info,warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_level(true)
        .try_init();
}

/// Production entry — invoked by `bin/main.rs`. Starts the Tauri shell.
/// Phase 1: Tauri starts but creates NO windows (windows array empty in tauri.conf.json).
/// The process stays alive via Tauri's run loop; Plans 04/05 spawn background tasks
/// inside the `setup()` closure.
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
