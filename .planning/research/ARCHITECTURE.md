# Architecture Research

**Domain:** Windows desktop achievement notification overlay (file-watcher driven, local-only, single-process)
**Researched:** 2026-05-07
**Confidence:** HIGH

---

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                        SOURCE LAYER                                  │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐          │
│  │ SteamAdapter   │  │GoldbergAdapter │  │CreamAPIAdapter │  ...     │
│  │ (legit VDF)    │  │ (JSON appdata) │  │ (JSON appdata) │          │
│  └───────┬────────┘  └───────┬────────┘  └───────┬────────┘          │
│          │                  │                   │                    │
│          └──────────────────┴───────────────────┘                   │
│                             │ RawUnlockEvent                         │
├─────────────────────────────┼────────────────────────────────────────┤
│                      WATCHER CORE                                    │
│              ┌──────────────▼──────────────┐                        │
│              │  Debouncer / Deduplicator    │                        │
│              │  (notify-debouncer-mini)     │                        │
│              └──────────────┬──────────────┘                        │
│                             │ Unlock(appId, achApiName, ts, source)  │
├─────────────────────────────┼────────────────────────────────────────┤
│                    ENRICHMENT LAYER                                  │
│   ┌──────────────────────── ▼ ──────────────────────────────────┐   │
│   │                  Schema Resolver                             │   │
│   │   (display name, description, icon path, rarity)            │   │
│   │   Local SQLite cache + lazy Steam Web API fetch              │   │
│   └──────────────────────── ┬ ───────────────────────────────── ┘   │
│                             │ EnrichedUnlock                         │
├─────────────────────────────┼────────────────────────────────────────┤
│                  ORCHESTRATION LAYER                                 │
│   ┌──────────┐   ┌──────────▼──────────┐   ┌─────────────────┐     │
│   │  Game    │   │  Notification Queue │   │  Persistent     │     │
│   │ Session  │──▶│  / Orchestrator     │──▶│  Store          │     │
│   │ Detector │   │  (burst management) │   │  (SQLite)       │     │
│   └──────────┘   └──────────┬──────────┘   └─────────────────┘     │
│                             │ ShowPopup(EnrichedUnlock)              │
├─────────────────────────────┼────────────────────────────────────────┤
│                    PRESENTATION LAYER                                │
│   ┌──────────────────────── ▼ ──────────────────────────────────┐   │
│   │  Popup Renderer (Tauri WebviewWindow)                        │   │
│   │  borderless · transparent · always-on-top · CSS animation    │   │
│   └─────────────────────────────────────────────────────────────┘   │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │  Companion Window (Tauri WebviewWindow)                      │   │
│   │  session achievement list · game header · rarity badges      │   │
│   └─────────────────────────────────────────────────────────────┘   │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │  Tray Icon (process anchor, show/hide companion)             │   │
│   └─────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘
```

---

## Component Responsibilities

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| **SteamAdapter** | Watch `Steam/appcache/stats/UserGameStatsSchema_<appid>.bin` timestamp; diff against persisted state to detect new unlocks | Watcher Core (emit RawUnlockEvent) |
| **GoldbergAdapter** | Watch `%APPDATA%\Goldberg SteamEmu Saves\<appid>\achievements.json`; diff JSON for `Achieved: true` fields | Watcher Core (emit RawUnlockEvent) |
| **CreamAPIAdapter** | Watch `%APPDATA%\CreamAPI\<appid>\` for stats/achievements file changes | Watcher Core (emit RawUnlockEvent) |
| **SmartSteamEmuAdapter** | Watch `%APPDATA%\SmartSteamEmu\<persona>\<appid>\` achievement files | Watcher Core (emit RawUnlockEvent) |
| **Watcher Core** | Registers all active source adapters; owns file-system watcher (notify crate); debounces (500ms window); deduplicates by (source, appId, achApiName) within session | Schema Resolver, Persistent Store |
| **Schema Resolver** | Given (appId, achApiName) → returns display name, description, icon path, rarity. Checks SQLite cache first; lazy-fetches Steam Web API on cache miss | Steam Web API (HTTP), Persistent Store (icon cache), Watcher Core |
| **Game Session Detector** | Polls Windows process list (WMI `Win32_Process` via wmi crate, 2s interval); reads Steam `libraryfolders.vdf` / `loginusers.vdf` to map process → appId; emits SessionStarted / SessionEnded events | Notification Queue, Companion Window, Watcher Core |
| **Notification Queue / Orchestrator** | Receives EnrichedUnlock events; manages display timing (one popup at a time, 500ms gap between queued items in a burst); emits ShowPopup to renderer | Popup Renderer, Companion Window, Persistent Store |
| **Popup Renderer** | Single Tauri WebviewWindow: `decorations(false)`, `transparent(true)`, `always_on_top(true)`. Receives ShowPopup command, animates in/out, plays sound | Notification Queue (listen for commands) |
| **Companion Window** | Second Tauri WebviewWindow: shown on SessionStarted, hidden on SessionEnded; lists current game's achievements (earned this session highlighted). Subscribes to EnrichedUnlock stream | Game Session Detector, Persistent Store |
| **Persistent Store** | SQLite via tauri-plugin-sql (sqlx feature). Three concerns: schema cache, icon blob cache, unlock history. Single DB file in Tauri app data dir | All components (read/write) |
| **Settings** | Minimal TOML or Tauri store plugin. Stores: watched paths override list, popup screen corner, popup duration. No theme knobs in v1 | All components that need user config |

---

## Source Adapter Interface

The following trait is the contract every source adapter must implement. Designing it now prevents Goldberg-specific assumptions leaking into Watcher Core.

```rust
// src/sources/mod.rs

use std::path::PathBuf;
use tokio::sync::mpsc;

/// A raw unlock event emitted by a source adapter before enrichment.
#[derive(Debug, Clone)]
pub struct RawUnlockEvent {
    pub app_id: u64,
    pub ach_api_name: String,
    pub timestamp: u64,       // Unix seconds; 0 if source does not record time
    pub source: SourceKind,
}

/// Identifies which adapter produced an event (for dedup and display).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceKind {
    SteamLegit,
    Goldberg,
    CreamApi,
    SmartSteamEmu,
    /// Future community-contributed adapters carry a stable string ID.
    Community(String),
}

/// Every source adapter implements this trait.
/// Watcher Core calls `start()` once; the adapter self-manages its watchers
/// and emits events onto the provided sender for the lifetime of the process.
#[async_trait::async_trait]
pub trait SourceAdapter: Send + Sync + 'static {
    /// Human-readable name, used in logs and UI source badges.
    fn name(&self) -> &str;

    /// Unique stable ID (matches SourceKind discriminant string).
    fn kind(&self) -> SourceKind;

    /// Filesystem roots this adapter watches. Watcher Core uses this
    /// to set up notify watchers; returning an empty vec means the adapter
    /// manages its own polling internally.
    fn watch_paths(&self) -> Vec<PathBuf>;

    /// Called once at startup. The adapter sends RawUnlockEvents onto `tx`
    /// for the lifetime of the process. Must not block.
    async fn start(&self, tx: mpsc::Sender<RawUnlockEvent>) -> anyhow::Result<()>;

    /// Called when a file event fires on one of the adapter's watch_paths.
    /// The adapter diffs current file state against its internal snapshot
    /// and sends any new unlock events onto `tx`.
    async fn on_file_changed(
        &self,
        path: PathBuf,
        tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()>;
}
```

**Design rationale:**
- `watch_paths()` keeps path registration in the adapter, not the core. A future Epic adapter can return `%APPDATA%\Epic\...` without touching Watcher Core.
- `on_file_changed()` separates "what changed" (adapter) from "how to watch" (core). The core only manages the notify machinery.
- `SourceKind::Community(String)` lets post-v1 adapters loaded at runtime (e.g. from a plugin directory) self-identify without requiring an enum variant change.
- Adapters own their own internal snapshot state. The core never knows about JSON vs VDF vs INI — it only knows `RawUnlockEvent`.

---

## Data Flow: Disk Change to Screen Popup

```
[DISK] achievement file modified
    │
    ▼  (notify crate, ReadDirectoryChangesW on Windows)
[SourceAdapter.on_file_changed()]
    │  reads file, diffs against last known state
    ▼
[RawUnlockEvent { app_id, ach_api_name, timestamp, source }]
    │
    ▼  (mpsc channel into Watcher Core)
[Debouncer]  ← 500ms quiet window, keyed on (source, app_id, ach_api_name)
    │  collapses duplicate writes (Steam writes stats file 2-3× per unlock)
    ▼
[Deduplicator]  ← checks Persistent Store: "was this already recorded?"
    │  drops if already in unlock_history for this session
    ▼
[Schema Resolver]
    │  checks SQLite schema_cache for (app_id, ach_api_name)
    │  cache hit → immediate
    │  cache miss → async fetch ISteamUserStats/GetSchemaForGame, write cache
    ▼
[EnrichedUnlock { ..raw_fields, display_name, description, icon_path, rarity }]
    │
    ├──▶ [Persistent Store]  writes to unlock_history table
    │
    ├──▶ [Companion Window]  appends to session list (via Tauri event emit)
    │
    └──▶ [Notification Queue]
             │  if queue empty → immediate
             │  if queue has items → wait for current popup dismiss + 500ms gap
             ▼
         [Popup Renderer] ← Tauri event: "show_popup" with EnrichedUnlock payload
             │  CSS keyframe animation in
             │  play sound (Tauri audio or <audio> element)
             │  auto-dismiss after ~4s (or on click)
             ▼
         [Popup Renderer] animates out
             │
             └──▶ [Notification Queue] signals "ready for next"
```

---

## Known Source File Paths (HIGH confidence from community tools)

| Source | Default Path | File / Format |
|--------|-------------|---------------|
| Steam (legit) | `Steam\appcache\stats\UserGameStatsSchema_<appid>.bin` | Binary VDF — schema only; unlock state in `Steam\userdata\<steamid>\<appid>\remote\` or detected via `appcache\stats` mtime change |
| Goldberg | `%APPDATA%\Goldberg SteamEmu Saves\<appid>\achievements.json` | JSON; each achievement has `Achieved: bool`, `CurProgress`, `MaxProgress`, `UnlockTime: unix` |
| CreamAPI | `%APPDATA%\CreamAPI\<appid>\` | INI or JSON depending on version; `saveindirectory` config can change path |
| SmartSteamEmu | `%APPDATA%\SmartSteamEmu\<persona>\<appid>\` | Per-persona folder; achievements stored alongside remote storage |

**Note on legit Steam detection:** Steam writes `UserGameStatsSchema_<appid>.bin` in `appcache\stats` and updates per-user achievement VDF data in `userdata\<steamid>\<appid>\`. Watching mtime on the appcache stats file is the reliable signal used by Achievement Watcher (xan105). The actual unlock state requires parsing the binary VDF, which can be done with the `keyvalues-serde` or `new-vdf-parser` Rust crates. This is the one adapter that requires VDF binary parsing — the others are plain JSON/INI.

---

## Storage Shape

### Recommendation: Single SQLite database

Use **one SQLite file** (`hallmark.db`) in the Tauri app data directory (`$APPDATA\Hallmark\`). Separate the concerns with distinct tables rather than separate files or DBs.

**Rationale:** All reads are local, latency is irrelevant, foreign keys between tables are valuable (icon cache referenced by schema cache), SQLite handles concurrent reads from the single-process context without contention. Flat JSON files lose transactional safety and are harder to query for the companion view.

### Schema

```sql
-- Achievement schema fetched from Steam Web API
CREATE TABLE schema_cache (
    app_id           INTEGER NOT NULL,
    ach_api_name     TEXT    NOT NULL,
    display_name     TEXT    NOT NULL,
    description      TEXT    NOT NULL DEFAULT '',
    icon_normal_url  TEXT,           -- CDN URL: cdn.akamai.steamstatic.com/...
    icon_gray_url    TEXT,           -- locked variant
    icon_cache_key   TEXT,           -- FK to icon_cache.cache_key
    global_percent   REAL,           -- rarity, if available
    fetched_at       INTEGER NOT NULL, -- unix timestamp
    PRIMARY KEY (app_id, ach_api_name)
);

-- Raw icon blobs downloaded from CDN
CREATE TABLE icon_cache (
    cache_key  TEXT    PRIMARY KEY,  -- SHA1 of URL or filename hash
    mime_type  TEXT    NOT NULL DEFAULT 'image/jpeg',
    data       BLOB    NOT NULL,
    fetched_at INTEGER NOT NULL
);

-- Every unlock ever seen by Hallmark (across all sessions)
CREATE TABLE unlock_history (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    app_id       INTEGER NOT NULL,
    ach_api_name TEXT    NOT NULL,
    source       TEXT    NOT NULL,   -- 'steam_legit' | 'goldberg' | etc.
    unlocked_at  INTEGER NOT NULL,   -- unix timestamp from source (or wall clock)
    session_id   TEXT    NOT NULL,   -- UUID per game session
    notified     INTEGER NOT NULL DEFAULT 0  -- 1 = popup was shown
);
CREATE INDEX idx_unlock_session ON unlock_history(session_id);
CREATE INDEX idx_unlock_app     ON unlock_history(app_id, ach_api_name);

-- Game sessions (enables cross-restart "earned this session" query)
CREATE TABLE sessions (
    session_id   TEXT    PRIMARY KEY,  -- UUID
    app_id       INTEGER NOT NULL,
    app_name     TEXT,
    started_at   INTEGER NOT NULL,
    ended_at     INTEGER             -- NULL while session active
);

-- User settings (key/value; avoids schema migration for new settings)
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

### Schema Cache TTL

- **Default TTL:** 7 days. Achievement schema (names, descriptions, icons) changes extremely rarely. Aggressive caching is correct.
- **Rarity (`global_percent`):** Cache with 24h TTL separately if fetched from SteamSpy, since percentages drift.
- **No TTL for icon blobs:** Icons never change. Evict by LRU if DB exceeds configurable size limit (default: 200MB).

---

## Schema / Icon Fetching Strategy

### Fetch trigger: lazy on first unknown (app_id, ach_api_name)

Do not eagerly pre-fetch on startup. The schema resolver fetches on first cache miss during the unlock path.

**Reasoning:**
- Pre-fetching all installed games' schemas at startup adds startup latency, burns API quota, and fetches schemas for games the user may never play during the session.
- Lazy fetch on first unlock is the worst-case latency scenario, but: the notification queue can show the unlock with a "loading..." icon while the fetch completes in the background (1–2s). This is acceptable UX for the first-ever unlock of a game.

### Fetch sequence

```
1. Schema Resolver receives (app_id, ach_api_name)
2. Check schema_cache WHERE app_id=? AND ach_api_name=? AND fetched_at > (now - 7 days)
3. Hit → return cached row immediately
4. Miss →
   a. Return partial EnrichedUnlock with api_name as display_name (allows popup to show immediately)
   b. Spawn async task: GET https://api.steampowered.com/ISteamUserStats/GetSchemaForGame/v2/?appid=<id>&key=<key>
   c. Parse response: for each achievement, upsert schema_cache
   d. Download icon URLs to icon_cache (background, non-blocking for popup)
   e. Emit "schema_updated" event → Popup Renderer and Companion Window re-render if visible
```

### Icon URL format (HIGH confidence)

The `GetSchemaForGame` API response contains `icon` and `icongray` fields with URLs of the form:
```
https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/<appid>/<hash>.jpg
```
Download and store as BLOB in `icon_cache`. Serve as `data:image/jpeg;base64,...` or via Tauri asset protocol to the renderer.

### No Steam API key required for schema

`GetSchemaForGame` works without an API key for public games. Including a key raises rate limits. In v1, operate keyless; document how to add a key in settings for power users.

---

## Process Model

### Single process, three windows

```
hallmark.exe
├── Rust backend (Tauri)
│   ├── Watcher Core (tokio task)
│   ├── Source Adapters × N (tokio tasks)
│   ├── Game Session Detector (tokio task, 2s poll)
│   ├── Schema Resolver (async, on-demand)
│   ├── Notification Queue (tokio task)
│   └── Persistent Store (SQLite, single connection pool)
│
├── WebviewWindow: "popup"
│   ├── decorations: false, transparent: true, always_on_top: true
│   ├── Starts hidden; shown/hidden per notification lifecycle
│   └── Position: user-configured corner (default: bottom-right)
│
├── WebviewWindow: "companion"
│   ├── Standard decorations, resizable
│   ├── Shown on SessionStarted, hidden on SessionEnded
│   └── Position: persisted between sessions
│
└── Tray Icon
    ├── Process anchor — app lives here when no game session active
    ├── Left-click: toggle companion window
    └── Right-click menu: "Settings", "Quit"
```

### Game lifecycle behavior

| Event | Popup Window | Companion Window | Tray |
|-------|-------------|-----------------|------|
| App starts, no game running | Hidden | Hidden | Visible |
| Game launches (SessionStarted) | Hidden (ready) | Shown/focused | Visible |
| Achievement unlocks | Animate in, then out | List updates | Unchanged |
| Game closes (SessionEnded) | Hidden | Hidden (not closed) | Visible |
| User clicks tray | — | Toggle show/hide | — |
| User quits | — | — | Removed |

**Note:** Companion window is hidden (not destroyed) on SessionEnded. This preserves window position for the next session and avoids re-initialization cost. The session history in SQLite means the companion can show "last session" data when shown manually between games.

---

## Recommended Project Structure

```
src-tauri/
├── src/
│   ├── main.rs                  # Tauri builder, plugin registration
│   ├── lib.rs                   # App state, command registration
│   │
│   ├── sources/
│   │   ├── mod.rs               # SourceAdapter trait, RawUnlockEvent, SourceKind
│   │   ├── steam_legit.rs       # SteamAdapter: appcache/stats watcher, VDF parser
│   │   ├── goldberg.rs          # GoldbergAdapter: JSON watcher
│   │   ├── cream_api.rs         # CreamAPIAdapter
│   │   └── smart_steam_emu.rs   # SmartSteamEmuAdapter
│   │
│   ├── watcher/
│   │   ├── mod.rs               # WatcherCore: orchestrates adapters, notify setup
│   │   ├── debouncer.rs         # 500ms debounce + session-scoped deduplication
│   │   └── events.rs            # RawUnlockEvent, EnrichedUnlock type defs
│   │
│   ├── schema/
│   │   ├── mod.rs               # SchemaResolver: cache lookup + fetch dispatch
│   │   ├── steam_api.rs         # HTTP client: GetSchemaForGame, icon download
│   │   └── cache.rs             # SQLite read/write helpers for schema_cache
│   │
│   ├── session/
│   │   ├── mod.rs               # GameSessionDetector: WMI process poll
│   │   ├── process_scanner.rs   # Win32 process list via wmi crate
│   │   └── steam_reader.rs      # Steam VDF reader: appid resolution
│   │
│   ├── queue/
│   │   └── mod.rs               # NotificationQueue: burst management, timing
│   │
│   ├── store/
│   │   ├── mod.rs               # PersistentStore: SQLite pool, migrations
│   │   ├── migrations/          # SQL migration files (numbered)
│   │   └── queries.rs           # Typed query helpers
│   │
│   └── commands.rs              # Tauri commands exposed to frontend
│
src/ (frontend)
├── windows/
│   ├── popup/                   # Popup window SPA
│   │   ├── App.tsx
│   │   └── PopupCard.tsx        # Achievement card component
│   └── companion/               # Companion window SPA
│       ├── App.tsx
│       ├── SessionHeader.tsx
│       └── AchievementList.tsx
│
├── lib/
│   ├── events.ts                # Tauri event listeners (typed)
│   └── store.ts                 # Frontend state (Zustand or nanostores)
│
└── assets/
    └── sounds/
        └── unlock.mp3           # Signature sound — designer-locked
```

---

## Build Order and Dependency Analysis

### Phase independence graph

```
[A] SourceAdapter trait + mock emitter     (independent — no deps)
[B] Popup Renderer window + CSS animation  (independent — mock events only)
[C] Companion Window UI                    (independent — mock session data)
[D] PersistentStore / SQLite schema        (independent — no UI deps)

[E] GoldbergAdapter              depends on [A] trait definition
[F] SteamAdapter (VDF)           depends on [A] trait definition (harder)
[G] CreamAPIAdapter              depends on [A] trait definition
[H] SmartSteamEmuAdapter         depends on [A] trait definition

[I] Watcher Core + Debouncer     depends on [A][E/F/G/H] adapters exist
[J] Game Session Detector        independent of watcher; depends on [D] (sessions table)
[K] Schema Resolver              depends on [D] (schema_cache table)
[L] Notification Queue           depends on [I][K]
[M] Full integration             depends on all of the above
```

### Suggested build sequence

**Parallel stream 1 (core data pipeline):**
1. Define `SourceAdapter` trait + `RawUnlockEvent` / `EnrichedUnlock` types — everything downstream depends on these shapes
2. Build `PersistentStore` migrations and typed helpers
3. Implement `GoldbergAdapter` (easiest: plain JSON, well-documented paths)
4. Implement `Watcher Core` + debouncer against Goldberg only
5. Implement `Schema Resolver` (lazy fetch, cache)
6. Implement `Notification Queue`

**Parallel stream 2 (UI, runs concurrently with stream 1):**
1. Build popup window with mock `EnrichedUnlock` events (hardcoded fixture)
2. Build companion window with mock session data
3. Wire real Tauri events once stream 1 reaches step 6

**Parallel stream 3 (session detection, runs concurrently):**
1. Implement `Game Session Detector` against real process list
2. Wire to companion show/hide behavior

**Sequenced last:**
- `SteamAdapter` (VDF binary parsing is the hardest adapter — build after all other pieces proven)
- `CreamAPIAdapter` + `SmartSteamEmuAdapter` (lower-priority, same pattern as Goldberg)
- End-to-end integration test with all adapters live

---

## Architectural Patterns

### Pattern 1: Event-driven pipeline with typed channel boundaries

**What:** Each layer communicates via typed mpsc channels (`RawUnlockEvent` → `EnrichedUnlock` → `PopupCommand`). No shared mutable state crosses layer boundaries.

**When to use:** Any time the pipeline is async and stages have different latencies (file IO, HTTP fetch, render).

**Trade-offs:** Slightly more boilerplate at stage boundaries; avoids coupling and makes each stage independently testable with mock senders.

```rust
// Watcher Core emits into schema resolver's inbox
let (raw_tx, raw_rx) = mpsc::channel::<RawUnlockEvent>(64);
let (enriched_tx, enriched_rx) = mpsc::channel::<EnrichedUnlock>(64);

// Schema Resolver consumes raw_rx, emits enriched_tx
tokio::spawn(schema_resolver.run(raw_rx, enriched_tx));
```

### Pattern 2: Adapter-owned file snapshot state

**What:** Each source adapter maintains its own HashMap of last-known achievement state. Watcher Core never sees the file contents — only unlock diffs.

**When to use:** When different sources have radically different file formats (JSON vs binary VDF vs INI).

**Trade-offs:** Adapter state is in-memory only; on restart, adapters re-read current file state as baseline (no false unlocks from pre-existing achievements). This is correct behavior — a cross-restart dedup exists in `unlock_history`.

### Pattern 3: Tauri command / event split for window communication

**What:** Rust → Frontend communication uses Tauri events (push). Frontend → Rust communication uses Tauri commands (request/response). Never poll from the frontend.

**When to use:** Always, for this app. Popup and companion never need to query Rust; they only need to receive events.

```rust
// Rust pushes to popup window
app_handle.emit_to("popup", "show_popup", &enriched_unlock)?;

// Frontend listens
listen("show_popup", |event| { /* animate in */ });
```

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Baking source-specific paths into Watcher Core

**What people do:** Hard-code `%APPDATA%\Goldberg SteamEmu Saves` directly in the core watcher loop.

**Why it's wrong:** Makes adding new adapters (CreamAPI, Epic, etc.) require editing core logic. Goldberg's path is configurable by the user anyway.

**Do this instead:** Each adapter declares its own `watch_paths()`. The core iterates adapters to collect paths.

### Anti-Pattern 2: Fetching schema synchronously on unlock

**What people do:** Block the unlock pipeline on a `GET /GetSchemaForGame` HTTP call before emitting to the notification queue.

**Why it's wrong:** First unlock of any game will stall for 1–3s. On a slow connection this is worse.

**Do this instead:** Emit a partial EnrichedUnlock immediately (api_name as display_name, placeholder icon). Fetch schema async, then emit a "schema_ready" update event to re-render both the popup (if still visible) and the companion list.

### Anti-Pattern 3: Using a separate background service process

**What people do:** Split into a "monitor service" process and a "UI process" with IPC between them.

**Why it's wrong:** Adds IPC complexity, two processes to keep alive, install/uninstall surface. Tauri's async Rust backend handles all background work within the single process cleanly.

**Do this instead:** All background tasks run as tokio spawned tasks inside the Tauri process. Tray icon keeps the process alive when no window is visible.

### Anti-Pattern 4: Treating Steam legit and Goldberg as identical

**What people do:** Assume both write the same JSON format to the same path.

**Why it's wrong:** Steam legit writes binary VDF to `appcache/stats/`; Goldberg writes JSON to AppData. Detection approaches and parsers are entirely different.

**Do this instead:** Separate adapter implementations. The adapter trait hides this difference from core.

### Anti-Pattern 5: Displaying a popup for every file-change event

**What people do:** Fire a popup directly from the file watcher callback.

**Why it's wrong:** Steam and some emulators write the stats file 2–4 times per single unlock (progress update, then final state). You get duplicate popups.

**Do this instead:** Debounce at the Watcher Core level (500ms quiet window); additionally deduplicate by checking `unlock_history` — if (app_id, ach_api_name) already exists with `notified = 1`, drop.

---

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| Steam Web API (`GetSchemaForGame`) | HTTP GET, lazy on cache miss, 7-day TTL | No key needed for public games; rate limit is 100k/day keyless |
| Windows Process API (WMI) | `wmi` Rust crate, 2s polling loop | More reliable than `CreateToolhelp32Snapshot` for sustained watching |
| Steam VDF files | `keyvalues-serde` or `new-vdf-parser` crate | Binary VDF only needed for legit Steam adapter |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| Source Adapter → Watcher Core | `mpsc::Sender<RawUnlockEvent>` | Adapters are fire-and-forget; core owns the receiver |
| Watcher Core → Schema Resolver | `mpsc::channel` | Schema resolver is a pipeline stage, not a service |
| Schema Resolver → Notification Queue | `mpsc::channel` | Enriched events flow downstream |
| Notification Queue → Popup Renderer | Tauri `emit_to("popup", ...)` | Cross-process boundary (Rust → WebView) |
| Notification Queue → Companion Window | Tauri `emit_to("companion", ...)` | Same pattern |
| Session Detector → Companion Window | Tauri `emit_to("companion", ...)` | SessionStarted / SessionEnded events |
| Any component → Persistent Store | Direct async function calls | Single-process, no IPC needed; SQLite handles concurrency |

---

## Scaling Considerations

This is a single-user local desktop app. Traditional scaling does not apply. The relevant "scale" axis is: how many watched directories and how many rapid successive unlocks.

| Scenario | Concern | Mitigation |
|----------|---------|-----------|
| User has 200+ games with Goldberg | Many watch paths registered with notify | notify uses ReadDirectoryChangesW recursively; batching paths under parent dirs where possible reduces handle count |
| Game unlocks 50 achievements at once (import, completion burst) | Notification queue back-pressure | Queue with 500ms inter-popup gap; show "X more queued" badge on popup after 3rd item |
| Icon cache grows unbounded | SQLite BLOB storage | Configurable max DB size (default 200MB); LRU eviction on icon_cache |
| VDF parse latency on startup (legit Steam, many games) | Blocking startup | Parse schemas lazily, only on first unlock event for a game |

---

## Sources

- [Achievement Watcher (xan105) — compatibility paths and debounce pattern](https://github.com/xan105/Achievement-Watcher)
- [Achievement Watchdog (50t0r25) — Goldberg path: `%APPDATA%\GSE saves\`](https://github.com/50t0r25/achievement-watchdog)
- [xan105/Achievement-Watcher Compatibility Wiki — emulator default paths](https://github.com/xan105/Achievement-Watcher/wiki/Compatibility)
- [Tauri Window Customization — decorations, transparent, always_on_top](https://v2.tauri.app/learn/window-customization/)
- [Tauri Multi-Window tutorial — WebviewWindowBuilder, tray positioner](https://tauritutorials.com/blog/creating-windows-in-tauri)
- [Tauri SQL plugin — sqlx SQLite for desktop](https://v2.tauri.app/plugin/sql/)
- [notify-debouncer-mini — 500ms default, dedup pattern](https://oneuptime.com/blog/post/2026-01-25-file-watcher-debouncing-rust/view)
- [Rust WMI crate for Win32_Process watching — 1s poll interval](https://users.rust-lang.org/t/watch-for-windows-process-creation-in-rust/98603)
- [Steam ISteamUserStats/GetSchemaForGame API](https://partner.steamgames.com/doc/webapi/isteamuserstats)
- [Steam achievement icon CDN URL format — cdn.akamai.steamstatic.com](https://steamcommunity.com/discussions/forum/7/458606248634580173/)
- [Goldberg emulator — AppData path and achievements.json location](https://gitlab.com/Mr_Goldberg/goldberg_emulator/-/merge_requests/20)
- [CreamAPI default path: `%APPDATA%\CreamAPI\<appid>\`](https://github.com/NaughtDZ/creamapi/blob/master/cream_api.ini)
- [SmartSteamEmu AppData path and per-persona folder structure](https://github.com/ndelaplane/Wafflez/blob/master/SmartSteamEmu.ini)
- [Rust Adapter Design Pattern — trait-based interface for extensible adapters](https://rust-unofficial.github.io/patterns/)
- [Steam appcache/stats UserGameStatsSchema_APPID.bin — binary VDF, new-vdf-parser crate](https://crates.io/crates/new-vdf-parser)

---

*Architecture research for: Hallmark — Windows achievement notification overlay*
*Researched: 2026-05-07*
