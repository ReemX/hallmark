# Phase 4: Polish & Distribution - Pattern Map

**Mapped:** 2026-05-09
**Files analyzed:** 28 (new + modified)
**Analogs found:** 26 / 28

Phase 4 is integration work, not novel engineering. Almost every new file has an
existing Phase 1-3 analog whose imports, error-handling, persistence, and IPC
shape it should copy verbatim. Two files (the GitHub Actions workflow + the
NSIS installer config) have NO in-repo analog and must lean on RESEARCH.md
patterns instead.

## File Classification

| New / Modified File                                | Role                | Data Flow            | Closest Analog                                    | Match Quality |
|----------------------------------------------------|---------------------|----------------------|---------------------------------------------------|---------------|
| `src-tauri/src/tray.rs`                            | controller (native) | event-driven         | `src-tauri/src/lib.rs::run()` setup() event-wiring | role-match    |
| `src-tauri/src/autostart.rs`                       | utility             | sync I/O (registry)  | `src-tauri/src/paths.rs` (registry reader pattern) | role-match    |
| `src-tauri/src/test_trigger.rs`                    | utility             | event-emit (mpsc)    | `src-tauri/src/sources/goldberg.rs` `RawUnlockEvent` synthesis | role-match |
| `src-tauri/src/first_run.rs`                       | utility + window    | request-response     | `src-tauri/src/ui.rs::create_companion_window`    | exact         |
| `src-tauri/src/settings_window.rs`                 | utility + window    | request-response     | `src-tauri/src/ui.rs::create_companion_window`    | exact         |
| `src-tauri/src/portable_mode.rs`                   | utility             | sync I/O             | `src-tauri/src/paths.rs::read_steam_install`      | role-match    |
| `src-tauri/src/updater_glue.rs`                    | service (async)     | request-response     | `src-tauri/src/schema/mod.rs::resolve` (spawn-async + emit pattern) | role-match |
| `src-tauri/src/store/queries.rs` (extended)        | data access         | CRUD                 | existing same file (`get_first_run_done` mirrors `is_completion_fired`) | exact |
| `src-tauri/src/lib.rs` (extended `setup()`)        | bootstrap           | event-wiring         | existing same file                                | exact         |
| `src-tauri/src/lib.rs::commands::AppState` (extended) | shared state     | shared mutable       | existing same file                                | exact         |
| `src-tauri/Cargo.toml` (deps add)                  | config              | n/a                  | existing same file                                | exact         |
| `src-tauri/tauri.conf.json` (extended)             | config              | n/a                  | existing same file                                | exact         |
| `src-tauri/capabilities/settings.json`             | config              | n/a                  | `src-tauri/capabilities/companion.json`           | exact         |
| `src-tauri/capabilities/wizard.json`               | config              | n/a                  | `src-tauri/capabilities/companion.json`           | exact         |
| `src-tauri/capabilities/companion.json` (extended) | config              | n/a                  | existing same file                                | exact         |
| `src-tauri/icons/tray.ico`                         | asset               | n/a                  | (no analog — visual asset)                        | none          |
| `assets/sfx/*.wav` (D-29 swap)                     | asset               | n/a                  | existing same files                               | exact         |
| `src/main-settings.tsx`                            | component (entry)   | request-response     | `src/main-companion.tsx`                          | exact         |
| `src/main-wizard.tsx`                              | component (entry)   | request-response     | `src/main-popup.tsx` (event-driven entry)         | role-match    |
| `src/components/SettingsSourceRow.tsx`             | component           | render               | `src/components/AchievementRow.tsx`               | exact         |
| `src/components/WizardSourceRow.tsx`               | component           | render               | `src/components/AchievementRow.tsx`               | exact         |
| `src/components/UpdateModal.tsx`                   | component           | event-driven         | `src/components/PopupCard.tsx` (Framer + IPC)     | role-match    |
| `src/styles/settings.css`                          | style               | n/a                  | `src/styles/companion.css`                        | exact         |
| `src/styles/shared.css` (optional)                 | style               | n/a                  | `src/styles/companion.css`                        | exact         |
| `settings.html`                                    | entry               | n/a                  | `popup.html`                                      | exact         |
| `wizard.html`                                      | entry               | n/a                  | `popup.html`                                      | exact         |
| `vite.config.ts` (extended)                        | config              | n/a                  | existing same file                                | exact         |
| `.github/workflows/release.yml`                    | CI config           | n/a                  | (no analog — first workflow)                      | none          |

## Pattern Assignments

### `src-tauri/src/tray.rs` (controller, event-driven)

**Analog:** `src-tauri/src/lib.rs::run()` setup() — for the AppHandle / event-handler shape; tray API itself is Tauri-2 native (RESEARCH Pattern 1).

**Imports pattern** (lib.rs lines 23-24):
```rust
use tauri::{Listener, Manager};
use tracing_subscriber::EnvFilter;
```
Tray adds `tauri::menu`, `tauri::tray`, `tauri::image::Image`. Use absolute crate paths
(no `use crate::*` star imports — codebase always names sub-modules explicitly,
e.g. `crate::store::queries::set_companion_prefs`).

**AppHandle clone pattern** (lib.rs line 163):
```rust
let app_handle = app.handle().clone();
```
Tray's `on_menu_event` closure receives `app: &AppHandle` directly; for spawned
async tasks, clone via `app.handle().clone()` BEFORE the move.

**Event-handler error swallow pattern** (lib.rs lines 312-353 — `app.listen("game-started", ...)`):
```rust
let _unlisten_started = app.listen("game-started", move |event: tauri::Event| {
    let payload: serde_json::Value = match serde_json::from_str(event.payload()) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "failed to parse game-started payload");
            return;
        }
    };
    // ...
});
```
Tray menu handlers swallow errors via `let _ = ...` and log via `tracing::warn!` —
NEVER unwrap, NEVER propagate. The closure returns `()`.

**tracing::info! logging convention** (multiple sites in lib.rs):
```rust
tracing::info!(adapter_count = adapters.len(), "Phase 3: 4-adapter pipeline configured");
```
Structured fields preferred over interpolation. Tray actions log via the same
pattern: `tracing::info!(action = "show_companion", "tray menu action");`.

**Quit-with-drain pattern** — NO in-repo analog (Phase 2 has no quit). Use RESEARCH
Pitfall 5: emit a "shutdown" event, await `popup_queue` join with 1-2s timeout,
THEN `app.exit(0)`.

---

### `src-tauri/src/autostart.rs` (utility, sync I/O)

**Analog:** `src-tauri/src/paths.rs` — same `#[cfg(target_os = "windows")]` registry-read pattern (read_steam_install reads `HKEY_LOCAL_MACHINE\SOFTWARE\Valve\Steam`).

**Cross-platform stub pattern** — Phase 1 paths.rs uses `#[cfg(target_os = "windows")]` blocks. RESEARCH Pattern 2 mirrors this:
```rust
#[cfg(target_os = "windows")]
use winreg::{enums::*, RegKey};

#[cfg(not(target_os = "windows"))]
pub fn is_enabled() -> anyhow::Result<bool> { Ok(false) }
```
Even though Hallmark is Windows-only per PROJECT.md, the cfg-gating pattern is
already established in paths.rs — autostart.rs follows it for compile-friendliness
on dev machines that might be macOS/Linux.

**Error mapping** (paths.rs uses `anyhow::Result<T>` throughout). Same here:
```rust
pub fn is_enabled() -> anyhow::Result<bool> { ... }
pub fn enable() -> anyhow::Result<()> { ... }
pub fn disable() -> anyhow::Result<()> { ... }
```
Match `std::io::ErrorKind::NotFound` → `Ok(false)` rather than propagating
(RESEARCH Pattern 2 lines 442-447). Other errors `Err(e.into())`.

**Logging on success** (paths.rs `tracing::info!` per discovered path). Autostart
follows: `tracing::info!(value = %value, "autostart enabled (HKCU\\...\\Run)");`.

---

### `src-tauri/src/test_trigger.rs` (utility, event-emit)

**Analog:** `src-tauri/src/sources/goldberg.rs` (constructs RawUnlockEvent and
sends via mpsc; goldberg implements the `tx.send(RawUnlockEvent {...}).await` shape
the test trigger imitates).

**Synthesizing RawUnlockEvent pattern** (sources/mod.rs lines 36-49 — struct definition):
```rust
pub struct RawUnlockEvent {
    pub app_id: u64,
    pub ach_api_name: String,
    pub timestamp: u64,
    pub source: SourceKind,
}
```
Test fixture (RESEARCH Pattern 3):
```rust
let evt = RawUnlockEvent {
    app_id: TEST_APP_ID,        // 480 (Spacewar)
    ach_api_name: TEST_API_NAME.into(),  // "HALLMARK_TEST_UNLOCK"
    timestamp: std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs(),
    source: SourceKind::Goldberg,
};
```

**mpsc send pattern** — adapters use `tx.send(evt).await` from async; tray
handlers are sync, so use `tx.blocking_send(evt)`. Both are valid; choose by
caller context. Schema/AppState clone of `raw_tx` is the seam (RESEARCH lines 11
+ 487-528).

**SchemaCache short-circuit (D-05)** — RESEARCH recommends "Option 1: pre-seed
the cache at startup" via `cache::upsert_schema`. Pattern from `schema/mod.rs::resolve`
(lines 140-165): build a `SchemaCacheRow`, call `store.with_conn(|c| cache::upsert_schema(c, &row))`.
Insert ONCE in `lib.rs::run()` after `schema_cache = SchemaCache::new(...)` —
idempotent INSERT OR REPLACE on `(480, "HALLMARK_TEST_UNLOCK")`.

---

### `src-tauri/src/first_run.rs` (utility + window builder)

**Analog:** `src-tauri/src/ui.rs::create_companion_window` (lines 63-79).

**Window builder pattern** (ui.rs lines 64-76):
```rust
pub fn create_companion_window(app: &AppHandle) -> tauri::Result<()> {
    let _win = WebviewWindowBuilder::new(app, "companion", WebviewUrl::App("index.html".into()))
        .title("Hallmark Companion")
        .decorations(false)
        .transparent(false)
        .always_on_top(false)
        .skip_taskbar(false)
        .focused(false)
        .resizable(true)
        .visible(false)
        .inner_size(480.0, 720.0)
        .min_inner_size(360.0, 480.0)
        .center()
        .build()?;
    tracing::info!("companion window built (480x720 logical, hidden until game-start)");
    Ok(())
}
```

**Phase 4 wizard adaptation** (per UI-SPEC: 480×560, fixed, no close button):
```rust
pub fn create_wizard_window(app: &AppHandle) -> tauri::Result<()> {
    let _win = WebviewWindowBuilder::new(app, "wizard", WebviewUrl::App("wizard.html".into()))
        .title("Welcome to Hallmark")
        .decorations(false)
        .transparent(false)
        .always_on_top(false)
        .skip_taskbar(false)
        .focused(true)              // wizard SHOULD focus on first launch
        .resizable(false)
        .visible(true)              // wizard shows immediately on first run
        .inner_size(480.0, 560.0)   // UI-SPEC § Surface Specifications
        .center()
        .build()?;
    tracing::info!("wizard window built (480x560 logical)");
    Ok(())
}
```

**First-run flag persistence** — RESEARCH Pattern 6 recommends reusing
existing `settings(key, value)` table. Pattern from `store/queries.rs` lines
85-110 (mark_completion_fired / is_completion_fired):
```rust
pub fn get_first_run_done(conn: &Connection) -> anyhow::Result<bool> {
    let result = conn.query_row(
        "SELECT value FROM settings WHERE key = 'first_run_done'",
        [],
        |r| r.get::<_, String>(0),
    );
    match result {
        Ok(v) => Ok(v == "1"),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

pub fn set_first_run_done(conn: &Connection) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('first_run_done', '1')",
        [],
    )?;
    Ok(())
}
```
NO new migration needed; the `settings` table from `001_initial.sql` (lines 34-37)
already supports this.

---

### `src-tauri/src/settings_window.rs` (utility + window builder)

**Analog:** Same as first_run.rs — `ui.rs::create_companion_window`.

**Phase 4 settings adaptation** (per UI-SPEC: 520×580, fixed, has close button):
```rust
pub fn open(app: &AppHandle) -> tauri::Result<()> {
    // Idempotent: re-open existing window if already created
    if let Some(w) = app.get_webview_window("settings") {
        w.show()?;
        w.set_focus()?;
        return Ok(());
    }
    let _win = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Hallmark Settings")
        .decorations(false)
        .transparent(false)
        .always_on_top(false)
        .skip_taskbar(false)
        .focused(true)
        .resizable(false)
        .visible(true)
        .inner_size(520.0, 580.0)   // UI-SPEC § Settings Window
        .center()
        .build()?;
    tracing::info!("settings window built (520x580 logical)");
    Ok(())
}
```

`get_webview_window` lookup pattern from `popup_queue.rs` line 162-166:
```rust
if let Some(popup) = app.get_webview_window("popup") {
    if !popup.is_visible().unwrap_or(true) {
        let _ = popup.show();
    }
}
```

---

### `src-tauri/src/portable_mode.rs` (utility, sync I/O)

**Analog:** `src-tauri/src/paths.rs::read_steam_install` — uses `dirs::data_dir()`
and `std::env::current_exe()` patterns (lib.rs lines 166-168 also exemplify
the `dirs` crate use):
```rust
let db_dir = dirs::data_dir()
    .ok_or_else(|| anyhow::anyhow!("data_dir unavailable"))?
    .join("Hallmark");
```

**Heuristic pattern** — RESEARCH Pattern 7. Returns `bool`, not `Result<bool>`,
because failure-to-detect defaults to non-portable (safest). Logged via
`tracing::info!`.

**`std::env::args()` parsing for `--silent`** (RESEARCH Pitfall 4) — single
expression, no plugin:
```rust
pub fn is_silent_launch() -> bool {
    std::env::args().any(|a| a == "--silent")
}
```
Stash on AppState alongside `portable_mode` flag.

---

### `src-tauri/src/updater_glue.rs` (service, async)

**Analog:** `src-tauri/src/schema/mod.rs::resolve` (lines 110-226) — async
function spawned via `tokio::spawn`, emits Tauri events on stage completion,
swallows errors via warn-and-continue.

**Spawn + emit pattern** (lib.rs lines 254-302 — `tauri::async_runtime::spawn` with
`AppHandle::clone` move):
```rust
let app_for_queue = app_handle.clone();
let store_for_queue = store.clone();
let session_for_queue = session_id.clone();
tauri::async_runtime::spawn(async move {
    popup_queue::run(
        app_for_queue, sink_rx, schema_for_queue, audio_arc,
        store_for_queue, session_for_queue, pid_for_queue,
    ).await;
});
```

**Updater wiring** (RESEARCH Pattern 4 lines 553-577) follows the exact same
shape: clone AppHandle, spawn async task, emit Tauri event on result. Skip
when `portable_mode::is_portable()`.

**Error handling** (schema/mod.rs lines 167-176):
```rust
match steam_api::fetch_global_pcts(&self.http, app_id).await {
    Ok(pcts) => { /* merge */ }
    Err(e) => {
        tracing::warn!(app_id, error = %e, "global pcts fetch failed; continuing without rarity")
    }
}
```
Updater equivalent: `Err(e) => tracing::warn!(error = %e, "update check failed")`.

**Tauri event emit** (schema/mod.rs lines 180-184):
```rust
let _ = tauri::Emitter::emit(
    &app,
    "schema-resolved",
    serde_json::json!({"app_id": app_id, "stage": "metadata"}),
);
```
Updater emits `update-available` with `{version: "x.y.z", notes: "..."}`.

---

### `src-tauri/src/store/queries.rs` (extended) (data access, CRUD)

**Analog:** Existing same file — `is_completion_fired` / `mark_completion_fired`
(lines 85-110).

**Settings-table key/value pattern** (queries.rs lines 85-110, copied above
in `first_run.rs` section). Apply to:
- `get_first_run_done` / `set_first_run_done` (D-14)
- `get_last_update_check` / `set_last_update_check` (Claude's discretion timestamp)

**Test pattern** (queries.rs lines 256-269):
```rust
#[test]
fn completion_fired_round_trip() {
    let s = fresh_store();
    s.with_conn(|c| {
        assert!(!is_completion_fired(c, 480).unwrap(), "fresh DB has no completion flag");
        mark_completion_fired(c, 480).unwrap();
        assert!(is_completion_fired(c, 480).unwrap(), "after mark, flag is true");
        Ok(())
    }).unwrap();
}
```
Phase 4 mirrors with `first_run_done_round_trip`.

**`fresh_store` test helper** (queries.rs lines 196-198):
```rust
fn fresh_store() -> SqliteStore {
    SqliteStore::open_in_memory().unwrap()
}
```

---

### `src-tauri/src/lib.rs` (extended setup()) (bootstrap, event-wiring)

**Analog:** Existing same file. Phase 4 adds blocks at the end of the existing
setup() closure, never restructures.

**`pub mod` ladder** (lib.rs lines 8-21):
```rust
pub mod paths;
pub mod sources;
pub mod store;
pub mod watcher;

// ---- Phase 2 modules ----
pub mod schema;
pub mod audio;
pub mod monitor;
pub mod popup_queue;
pub mod ui;
pub mod game_detect;
```
Phase 4 appends:
```rust
// ---- Phase 4 modules ----
pub mod tray;
pub mod autostart;
pub mod test_trigger;
pub mod first_run;
pub mod settings_window;
pub mod portable_mode;
pub mod updater_glue;
```

**AppState extension pattern** (lib.rs lines 32-43):
```rust
pub mod commands {
    pub struct AppState {
        pub store: Arc<crate::store::SqliteStore>,
        pub schema: crate::schema::SchemaCache,
        pub session_id: String,
    }
}
```
Phase 4 adds fields:
```rust
pub struct AppState {
    pub store: Arc<crate::store::SqliteStore>,
    pub schema: crate::schema::SchemaCache,
    pub session_id: String,
    // Phase 4 additions
    pub raw_tx: tokio::sync::mpsc::Sender<crate::sources::RawUnlockEvent>,  // D-04 test inject
    pub portable_mode: bool,                                                // D-23
    pub silent_launch: bool,                                                // D-08
    pub pending_update: Arc<tokio::sync::Mutex<Option<tauri_plugin_updater::Update>>>,  // D-18
    pub cached_discovery: crate::paths::DiscoveredPaths,                    // wizard + settings rescan
}
```

**`invoke_handler` extension pattern** (lib.rs lines 157-161):
```rust
.invoke_handler(tauri::generate_handler![
    commands::get_companion_state,
    commands::set_companion_prefs_cmd,
    commands::get_companion_prefs_cmd,
])
```
Phase 4 appends new commands to this list:
```rust
.invoke_handler(tauri::generate_handler![
    commands::get_companion_state,
    commands::set_companion_prefs_cmd,
    commands::get_companion_prefs_cmd,
    // Phase 4
    commands::rescan_paths,
    commands::install_pending_update,
    commands::wizard_dismiss,
    commands::open_settings_window,
])
```

**Plugin registration (Tauri 2 pattern)** — RESEARCH Pattern 4. Add to builder
chain BEFORE `.setup()`:
```rust
.plugin(tauri_plugin_updater::Builder::new().build())
```

**Setup() ordering** (lib.rs lines 162-356) — Phase 4 inserts:
- After step 3 (path discovery): cache `discovery` clone for AppState
- After step 5 (adapters): clone `raw_tx` for AppState (test_trigger seam)
- After step 9 (windows created): `settings_window::pre-warm? NO — lazy`
- After step 10 (AppState management): seed test fixture into schema_cache
- After step 14 (game-started listener): build tray icon, register updater task,
  conditionally open first-run wizard

---

### `src-tauri/Cargo.toml` (extended) (config)

**Analog:** Existing same file.

**Dependency ordering pattern** — Cargo.toml groups by Phase. Phase 4 adds:
```toml
[dependencies]
# ... existing ...

# Phase 4: auto-updater
tauri-plugin-updater   = "2.10"

# Phase 4: HKCU registry — winreg already pinned for Phase 1 path discovery
# (see [target.'cfg(target_os = "windows")'.dependencies] below)
```

**Frontend deps (package.json)**:
```json
"@tauri-apps/plugin-updater": "^2"
```

---

### `src-tauri/tauri.conf.json` (extended) (config)

**Analog:** Existing same file (lines 1-26).

**Bundle activation flip** (existing line 19: `"active": false`). Phase 4:
```json
{
  "bundle": {
    "active": true,                       // flipped from false
    "createUpdaterArtifacts": true,       // generates .sig files
    "targets": ["nsis"],                  // explicit; portable .zip is custom CI step
    "category": "Utility",
    "shortDescription": "PSN/Xbox-grade achievement popups for PC gaming",
    "longDescription": "PSN/Xbox-grade achievement popups for PC gaming.",
    "icon": ["icons/icon.ico"],
    "windows": {
      "nsis": {
        "installMode": "perUser"          // D-22 — no UAC
      }
    }
  },
  "plugins": {
    "updater": {
      "pubkey": "<paste output of `tauri signer generate` public key>",
      "endpoints": [
        "https://github.com/<owner>/<repo>/releases/latest/download/latest.json"
      ]
    }
  }
}
```

**CSP** (existing line 15): connect-src already permits `https://api.steampowered.com`.
Updater needs `https://github.com` + `https://objects.githubusercontent.com` (release-asset
CDN). Append to `connect-src`.

---

### `src-tauri/capabilities/settings.json` (config)

**Analog:** `src-tauri/capabilities/companion.json` (verified existing file).

**Full file copy with adaptations** (companion.json lines 1-19):
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "settings-capability",
  "description": "Settings window — read-only paths panel, update check, about. Custom drag region.",
  "windows": ["settings"],
  "permissions": [
    "core:default",
    "core:event:allow-listen",
    "core:event:allow-unlisten",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-close",
    "core:window:allow-start-dragging",
    "updater:default"
  ]
}
```
Drop `core:window:allow-set-size` / `allow-set-position` / `allow-minimize`
(Settings is fixed-size).

---

### `src-tauri/capabilities/wizard.json` (config)

**Analog:** Same — `companion.json`.

**Adaptation** — wizard does NOT need updater capability:
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "wizard-capability",
  "description": "First-run welcome wizard. Shows path-discovery results.",
  "windows": ["wizard"],
  "permissions": [
    "core:default",
    "core:event:allow-listen",
    "core:event:allow-unlisten",
    "core:window:allow-show",
    "core:window:allow-close",
    "core:window:allow-start-dragging"
  ]
}
```

---

### `src-tauri/capabilities/companion.json` (extended) (config)

**Analog:** Existing same file.

**Phase 4 adds permissions** for update-modal install path:
```json
"permissions": [
  "core:default",
  // ... existing ...
  "updater:default",                        // D-18 modal "Install" button
  "core:webview:allow-create-webview-window" // for opening Settings/wizard from companion (if applicable)
]
```

---

### `src/main-settings.tsx` (component entry)

**Analog:** `src/main-companion.tsx` (verified existing file).

**Imports pattern** (main-companion.tsx lines 1-13):
```typescript
import { useEffect, useState, useCallback } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useGameSession } from "./hooks/useGameSession";
// ... components ...
import "./styles/companion.css";
import type { AchievementSchema } from "./types";
```

**Tauri-internals guard** (main-companion.tsx line 40):
```typescript
if (!("__TAURI_INTERNALS__" in window)) return; // browser preview — Tauri APIs unavailable
```
Use this guard in EVERY useEffect that calls `invoke` or `listen`.

**invoke + Promise.all pattern** (main-companion.tsx lines 46-54):
```typescript
Promise.all([
  invoke<CompanionState>("get_companion_state", { app_id: appId }),
  invoke<CompanionPrefs | null>("get_companion_prefs_cmd", { app_id: appId }),
])
.then(([s, p]) => { setState(s); setPrefs(p ?? defaultPrefs); })
.catch((e) => setError(String(e)));
```
Settings adapts: `invoke<DiscoveredPathsView>("rescan_paths")` then render rows.

**Render pattern** (main-companion.tsx lines 91-167):
```typescript
return (
  <div className="companion-shell">
    <CompanionHeader gameName="..." sessionEarned={...} />
    <div className="companion-controls">...</div>
    <div className="companion-list" role="list">...</div>
  </div>
);
```
Settings adapts to `.settings-shell` with `<SettingsHeader />`, sections.

**createRoot pattern** (main-companion.tsx lines 180-181):
```typescript
const root = document.getElementById("root");
if (root) createRoot(root).render(<CompanionRoot />);
```
Identical in main-settings.tsx.

---

### `src/main-wizard.tsx` (component entry)

**Analog:** `src/main-popup.tsx` (verified existing file) — for the simpler
"single state + listen" pattern (more like wizard than companion).

**Listener pattern** (main-popup.tsx lines 11-19):
```typescript
useEffect(() => {
  if (!("__TAURI_INTERNALS__" in window)) return;
  const unShow = listen<PopupPayload>("popup-show", (e) => setPayload(e.payload));
  const unHide = listen("popup-hide", () => setPayload(null));
  return () => {
    unShow.then((u) => u());
    unHide.then((u) => u());
  };
}, []);
```
Wizard simpler still: `invoke<DiscoveredPathsView>("get_discovered_paths")` once
on mount. Buttons "Get started" / "Continue anyway" call `invoke("wizard_dismiss")`.

---

### `src/components/SettingsSourceRow.tsx` (component, render)

**Analog:** `src/components/AchievementRow.tsx` (verified existing file).

**Component shape** (AchievementRow.tsx lines 5-48): props-based, motion.div root,
className composes from boolean state, child layout (icon | text).

**Adaptation** for SettingsSourceRow:
```tsx
import type { SourceStatus } from "../types";
export function SettingsSourceRow({ name, found }: { name: string; found: boolean }) {
  return (
    <div className={`source-row ${found ? "found" : "not-found"}`} role="listitem">
      <span className="source-mark" aria-hidden>{found ? "✓" : "✗"}</span>
      <span className="source-name">{found ? name : `${name} — not detected`}</span>
    </div>
  );
}
```
Per UI-SPEC § Settings Window — found ✓ in accent, not-found ✗ in text-secondary.

---

### `src/components/WizardSourceRow.tsx` (component, render)

**Analog:** Same as SettingsSourceRow — `AchievementRow.tsx`.

Mostly identical to SettingsSourceRow but with per-source explanatory text
when not-found (e.g., "Steam — libraryfolders.vdf not found"). UI-SPEC §
Copywriting Contract enumerates the exact strings.

---

### `src/components/UpdateModal.tsx` (component, event-driven)

**Analog:** `src/components/PopupCard.tsx` (verified existing file) — for the
Framer Motion pattern + IPC payload shape.

**Framer Motion pattern** (PopupCard.tsx lines 5, 27-32):
```tsx
import { motion, useReducedMotion } from "framer-motion";

const SPRING = { type: "spring" as const, stiffness: 380, damping: 28, mass: 0.9 };

return (
  <motion.div
    className={`popup-pill tier-${payload.tier}`}
    initial={reduceMotion ? { opacity: 0 } : { x: 480, opacity: 0 }}
    animate={reduceMotion ? { opacity: 1 } : { x: 0, opacity: 1 }}
    exit={reduceMotion ? { opacity: 0 } : { x: 0, y: -16, opacity: 0 }}
    transition={reduceMotion ? { duration: 0.15 } : SPRING}
  >
```

**Modal adaptation** per UI-SPEC § Update Modal: `fade-in + scale 0.96→1.0`
over 200ms, opposite for exit:
```tsx
const FADE_SCALE = { duration: 0.2, ease: "easeOut" };
return (
  <motion.div
    className="update-modal-backdrop"
    initial={{ opacity: 0 }}
    animate={{ opacity: 1 }}
    exit={{ opacity: 0 }}
    transition={FADE_SCALE}
  >
    <motion.div
      className="update-modal-card"
      initial={{ opacity: 0, scale: 0.96 }}
      animate={{ opacity: 1, scale: 1.0 }}
      exit={{ opacity: 0, scale: 0.96 }}
      transition={FADE_SCALE}
    >
      {/* Heading: "Update available" */}
      {/* Version badge: "v{new_version}" */}
      {/* Release notes (truncated) */}
      {/* "Read full release notes on GitHub" link */}
      {/* Buttons: "Install and Restart Hallmark" / "Later" */}
    </motion.div>
  </motion.div>
);
```

**Listener pattern in companion** (main-popup.tsx lines 11-19) — main-companion.tsx
extends to listen for `update-available` event:
```typescript
const unUpdate = listen<UpdateInfo>("update-available", (e) => setUpdateInfo(e.payload));
```

---

### `src/styles/settings.css` (style)

**Analog:** `src/styles/companion.css` (verified existing file).

**Color tokens to copy** (companion.css lines 2-3, 22, 29, 37, 60, 80, 84, 110-112):
- Background `#111114`
- Cards `#1C1C21`
- Earned tint `#1A2030`
- Accent `rgba(120, 220, 255, 0.85)`
- Destructive `#E05252`
- Text primary `#F0F0F5`
- Text secondary `rgba(240, 240, 245, 0.55)`

**Header pattern** (companion.css lines 11-29):
```css
.companion-header {
  height: 48px; padding: 0 16px; box-sizing: border-box;
  background: #111114; border-bottom: 1px solid rgba(255,255,255,0.06);
  display: flex; align-items: center; gap: 12px;
  user-select: none;
}
.companion-close {
  width: 28px; height: 28px; border-radius: 50%; border: none;
  background: transparent; color: #F0F0F5; font-size: 18px; cursor: pointer;
}
.companion-close:hover { background: #E05252; }
```
Settings reuses identically (per UI-SPEC inheritance rule).

**Pill button pattern** (companion.css lines 34-42 — filter chip):
```css
.filter-chip {
  padding: 6px 12px; border-radius: 16px; border: 1px solid transparent;
  background: #1C1C21; color: #F0F0F5;
  font-size: 14px; font-weight: 400; cursor: pointer;
}
```
Settings "Rescan" + "Check for updates" buttons follow this shape (per UI-SPEC
§ Settings Window: padding 6px 16px, background #1C1C21).

**Skeleton pattern** (companion.css lines 95-107):
```css
.skeleton-line {
  height: 12px; border-radius: 4px;
  background: linear-gradient(90deg, #1C1C21 0%, #2A2A30 50%, #1C1C21 100%);
  background-size: 200% 100%;
  animation: skeleton-pulse 1.5s ease-in-out infinite;
}
@keyframes skeleton-pulse { ... }
```
Settings rescan-in-progress reuses.

---

### `settings.html` and `wizard.html` (entry HTML)

**Analog:** `popup.html` (verified existing file).

**Full file pattern** (popup.html lines 1-14):
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Hallmark Popup</title>
    <link rel="stylesheet" href="/src/styles/popup.css" />
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main-popup.tsx"></script>
  </body>
</html>
```
Adapt: title + CSS path + script path. settings.html → `/src/styles/settings.css`
+ `/src/main-settings.tsx`. wizard.html → reuse settings.css (UI-SPEC permits) +
`/src/main-wizard.tsx`.

---

### `vite.config.ts` (extended)

**Analog:** Existing same file.

**rollupOptions.input pattern** (vite.config.ts lines 14-20):
```typescript
rollupOptions: {
  input: {
    companion: resolve(__dirname, "index.html"),
    popup: resolve(__dirname, "popup.html"),
  },
},
```
Phase 4 adds:
```typescript
rollupOptions: {
  input: {
    companion: resolve(__dirname, "index.html"),
    popup: resolve(__dirname, "popup.html"),
    settings: resolve(__dirname, "settings.html"),
    wizard: resolve(__dirname, "wizard.html"),
  },
},
```

---

### `.github/workflows/release.yml` (CI config)

**No analog in repo.** Use RESEARCH § Code Examples lines 922-1004 verbatim
(full YAML provided). Key constants:
- Trigger: `tags: ['v*.*.*']` + `workflow_dispatch`
- Runner: `windows-latest`
- Steps: pnpm setup → Node setup → Rust toolchain → cache → install → tauri-action → portable-zip pwsh step → gh release upload
- Env vars: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- Commented-out placeholders for D-24 future code-signing

### `assets/sfx/*.wav` (D-29 swap)

**No code analog.** Asset replacement only — `audio.rs` lines 48-69 reference
`popup-standard.wav`, `popup-rare.wav`, `popup-100pct.wav`. Replacements
must:
- Match filename exactly (drop-in)
- WAV PCM 16-bit @ 44.1 or 48kHz (RESEARCH Pitfall 9)
- Decode-validate at startup (audio.rs lines 60-69 already enforces this)

D-28 path: procedural via `gen_sfx.exe` (root of repo) is the recommendation.

---

## Shared Patterns

### Authentication / Authorization
**Not applicable** — Hallmark is a single-user desktop app with no auth surface.
Tauri capabilities are the only access-control mechanism (file: `src-tauri/capabilities/*.json`).

### Error Handling
**Source:** `src-tauri/src/schema/mod.rs` + `src-tauri/src/popup_queue.rs` (warn-and-continue pattern)
**Apply to:** ALL Phase 4 Rust files (tray.rs, autostart.rs, test_trigger.rs, updater_glue.rs, settings_window.rs, first_run.rs, portable_mode.rs)

**Pattern:**
```rust
match risky_call() {
    Ok(v) => { /* happy path */ }
    Err(e) => {
        tracing::warn!(error = %e, "context-specific message");
        // continue OR return Ok(()) OR `let _ = ...`
    }
}
```
NEVER `unwrap()` outside of tests. NEVER propagate errors from Tauri-event handlers
(closures returning `()`). Functions returning `anyhow::Result<T>` use `?` for
unrecoverable errors only.

### Tauri Command Pattern
**Source:** `src-tauri/src/lib.rs::commands` module (lines 32-96)
**Apply to:** All Phase 4 commands (`rescan_paths`, `install_pending_update`, `wizard_dismiss`, `open_settings_window`)

**Pattern** (lib.rs lines 56-77):
```rust
#[tauri::command]
pub fn get_companion_state(
    app_id: u64,
    state: tauri::State<'_, AppState>,
) -> Result<CompanionState, String> {
    let session_id = state.session_id.clone();
    let schema_list = state.schema.list_for_app(app_id);
    let earned = state.store.with_conn(|c| -> anyhow::Result<HashMap<String, i64>> {
        // ... rusqlite ...
    }).map_err(|e| e.to_string())?;
    Ok(CompanionState { app_id, schema: schema_list, earned, session_id })
}
```
Key conventions:
- Command fns are `pub`, not `pub(crate)` — proc-macro requires public visibility
- Argument types serde-deserialize from JSON IPC; struct args MUST derive `Deserialize`
- Return `Result<T, String>` — Tauri commands cannot return `anyhow::Error` directly
- `.map_err(|e| e.to_string())` is the canonical conversion at the boundary
- Async commands use `async fn` and `tauri::State<'_, AppState>` lifetime

### Tauri Event Emit Pattern
**Source:** `src-tauri/src/popup_queue.rs::emit_celebration` + `src-tauri/src/schema/mod.rs::resolve`
**Apply to:** updater_glue.rs (`update-available`), tray.rs (`open-settings`, `tray-action`), first_run.rs (`wizard-results`)

**Pattern A — typed payload to specific window** (popup_queue.rs line 167):
```rust
let _ = app.emit_to("popup", "popup-show", &payload);  // payload: serde::Serialize
```

**Pattern B — broadcast event** (schema/mod.rs lines 180-184):
```rust
let _ = tauri::Emitter::emit(
    &app,
    "schema-resolved",
    serde_json::json!({"app_id": app_id, "stage": "metadata"}),
);
```
Choose A when one specific window cares; B when multiple windows might listen
or the payload is dynamic JSON.

### State Persistence (settings table)
**Source:** `src-tauri/src/store/queries.rs::is_completion_fired` / `mark_completion_fired` (lines 85-110)
**Apply to:** `first_run_done` (D-14), `last_update_check` (Claude's discretion timestamp)

**Pattern:** key-value rows in existing `settings` table from `001_initial.sql`.
NO new migration. Pattern is already documented above in `first_run.rs` section.

### Logging
**Source:** Used everywhere (lib.rs, schema/mod.rs, popup_queue.rs, watcher/mod.rs, paths.rs)
**Apply to:** ALL Phase 4 Rust files

**Conventions:**
- `tracing::info!` for milestones (window built, task spawned, action taken)
- `tracing::warn!` for recoverable errors (with `error = %e` field)
- `tracing::debug!` for noisy diagnostics (per-event traces)
- `tracing::trace!` for hot-loop diagnostics
- ALWAYS use structured fields, never `format!` interpolation in the message
  - GOOD: `tracing::info!(app_id, ach = %ach_name, "UNLOCK")`
  - BAD:  `tracing::info!("UNLOCK app_id={} ach={}", app_id, ach_name)`

### React Component IPC Pattern
**Source:** `src/main-companion.tsx` + `src/main-popup.tsx`
**Apply to:** main-settings.tsx, main-wizard.tsx, components/UpdateModal.tsx

**Conventions:**
- Wrap `invoke` / `listen` in `if (!("__TAURI_INTERNALS__" in window)) return` guard for browser preview
- Cleanup listeners in useEffect return:
  ```typescript
  return () => {
    unShow.then((u) => u());
    unHide.then((u) => u());
  };
  ```
- Type IPC payloads in `src/types.ts` — extend with `DiscoveredPathsView`, `UpdateInfo`,
  `FirstRunState`, `SourceStatus` per Phase 4 RESEARCH lines 311

### CSS Inheritance from Phase 2
**Source:** `src/styles/companion.css` (verified)
**Apply to:** All Phase 4 frontend files (settings.css, optionally shared.css)

Per UI-SPEC: NO new design tokens. Phase 4 reuses Phase 2's color, typography,
spacing, and animation tokens. Concrete excerpts above in `src/styles/settings.css`
section.

## No Analog Found

Files with no close match in the codebase (planner should use RESEARCH.md patterns instead):

| File                           | Role        | Data Flow | Reason |
|--------------------------------|-------------|-----------|--------|
| `.github/workflows/release.yml` | CI config  | n/a       | First CI workflow in the repo. Use RESEARCH § Code Examples lines 922-1004 verbatim. |
| `src-tauri/icons/tray.ico`     | asset       | n/a       | Visual asset; design follows UI-SPEC § Surface Specifications "Tray Menu" guidance (monochrome, 16×16 + 32×32). |
| `src-tauri/tauri.conf.json` `bundle.windows.nsis` block | config | n/a | NSIS-specific keys are new to the repo. Pattern from RESEARCH Pattern 4 lines 587-590 + Tauri 2 docs. |

## Metadata

**Analog search scope:**
- `src-tauri/src/` (all `.rs` files)
- `src-tauri/capabilities/` (existing JSON capability files)
- `src-tauri/src/store/migrations/` (SQL files for settings-table reuse pattern)
- `src/` (frontend React entry points + components + styles)
- Repo root (HTML entry files, vite.config.ts, package.json, Cargo.toml)

**Files scanned (Read directly):** 25
- `src-tauri/src/lib.rs` (full)
- `src-tauri/src/ui.rs` (full)
- `src-tauri/src/sources/mod.rs` (full)
- `src-tauri/src/watcher/mod.rs` (full)
- `src-tauri/src/store/queries.rs` (full)
- `src-tauri/src/store/mod.rs` (full)
- `src-tauri/src/store/migrations/001_initial.sql` (full)
- `src-tauri/src/store/migrations/002_schema_cache.sql` (full)
- `src-tauri/src/schema/mod.rs` (full)
- `src-tauri/src/schema/cache.rs` (lines 1-100)
- `src-tauri/src/popup_queue.rs` (lines 1-220)
- `src-tauri/src/audio.rs` (lines 1-90)
- `src-tauri/src/paths.rs` (lines 1-80)
- `src-tauri/Cargo.toml` (full)
- `src-tauri/tauri.conf.json` (full)
- `src-tauri/capabilities/companion.json` (full)
- `src/main-companion.tsx` (full)
- `src/main-popup.tsx` (full)
- `src/components/AchievementRow.tsx` (full)
- `src/components/PopupCard.tsx` (full)
- `src/components/CompanionHeader.tsx` (full)
- `src/components/EmptyState.tsx` (full)
- `src/components/FilterBar.tsx` (full)
- `src/hooks/useGameSession.ts` (full)
- `src/styles/companion.css` (lines 1-130)
- `index.html`, `popup.html` (full)
- `vite.config.ts`, `package.json` (full)

**Pattern extraction date:** 2026-05-09

**Key findings:**
- Phase 4 is integration work with strong Phase 1-3 analogs for almost every file.
- Two genuinely new bits (CI workflow + NSIS config) lean on RESEARCH.md.
- The `settings(key, value)` table from `001_initial.sql` is the canonical
  state-persistence vehicle for `first_run_done` + `last_update_check` —
  NO new migration needed, matches `is_completion_fired` precedent.
- The `raw_tx` clone for D-04 test injection requires extending the existing
  `commands::AppState` struct in `src-tauri/src/lib.rs` lines 39-43.
- Phase 4 frontend strictly inherits Phase 2 CSS tokens per UI-SPEC.
