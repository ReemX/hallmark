# Phase 3: Remaining Source Adapters - Research

**Researched:** 2026-05-09
**Domain:** Steam binary VDF parsing (legitimate Steam achievement state) + CreamAPI INI achievement state + SmartSteamEmu binary `stats.bin` achievement state + N-source cross-source dedup verification + path-discovery extension to enumerate three new adapter trees, all wired into Phase 1's existing `SourceAdapter` trait + `WatcherCore` + `run_pipeline` + `CrossSourceDedup` pipeline without restructuring.
**Confidence:** MEDIUM (HIGH on Steam binary VDF — empirically inspected on this machine; HIGH on CreamAPI INI schema — confirmed against two canonical OSS parsers; MEDIUM on SmartSteamEmu — two competing canonical formats exist (`stats.bin` per Achievement-Watcher, `<appid>/User/Achievements.ini` per Hydra); LOW-MEDIUM on file-write atomicity for legit Steam — observable behaviour, not documented contract.)

## Summary

Phase 3 closes the adapter coverage gap left after Phase 1 (Goldberg only). Three new `SourceAdapter` implementations slot into the **existing** Phase 1 pipeline — `WatcherCore::run_watcher`, `run_pipeline`, `CrossSourceDedup`, `SqliteStore::record_unlock`, `idx_unlock_dedup` UNIQUE INDEX — with **no architectural changes** required. The pipeline was designed in Plan 01-04 to accept `Vec<Arc<dyn SourceAdapter>>` of arbitrary length and route prefix-matched events back to the owning adapter, and Plan 01-05's `CrossSourceDedup` keys on `(app_id, ach_api_name)` so it generalizes from 2 adapters to N.

The work decomposes into four concerns: **(1)** a Steam-legit adapter that watches `<SteamPath>\appcache\stats\UserGameStats_<userid>_<appid>.bin` (binary VDF), parses the binary KeyValues format into a per-stat-slot achievement state, and consults the per-app `UserGameStatsSchema_<appid>.bin` to map numeric stat indices back to Steam achievement API names; **(2)** a CreamAPI adapter that watches `%APPDATA%\CreamAPI\<appid>\stats\CreamAPI.Achievements.cfg` (INI file with `[ACH_API_NAME]` sections containing `achieved=true|false` and `unlocktime=<unix>`); **(3)** a SmartSteamEmu adapter that watches `%APPDATA%\SmartSteamEmu\<appid>\stats.bin` (24-byte-record binary file with CRC32-of-API-name as the achievement key, requiring a CRC32→API-name reverse lookup populated from a known-API-names list); **(4)** extending `paths::DiscoveredPaths` with three new fields (`steam_legit_userdata_ids`, `cream_api_appids`, `sse_appids`) and corresponding `discover()` enumeration logic that **does not depend on the Steam Web API** (per project constraint).

**Primary recommendation:** Hand-roll a focused **binary KeyValues VDF reader** (~250 LoC) for the Steam-legit adapter rather than depending on `steam-vdf-parser 0.1.1` — that crate's README explicitly targets `shortcuts.vdf` / `appinfo.vdf` / `packageinfo.vdf` and does not document UserGameStats compatibility. The format is small (8 type-tag bytes — 0x00 Object, 0x01 String, 0x02 Int32, 0x03 Float, 0x07 UInt64, 0x08 ObjectEnd are sufficient for both UserGameStats and UserGameStatsSchema files based on empirical inspection of the local Steam install). The reader produces a `keyvalues_parser::Vdf`-shaped tree which fits the existing parsing patterns in `paths.rs`. For CreamAPI, parse the INI with the simple line-oriented parser pattern Hydra uses (10 LoC) — do **not** add a new INI crate; for SmartSteamEmu, the existing `crc32fast` (or `crc 3.x`) crate provides CRC32; the API-name reverse-lookup is built once at session start by computing CRC32 over every API name discovered from the schema file, schema cache, or Goldberg companion file. Cross-source dedup as written in Phase 1 is **already correct for N adapters** — no change needed except to verify a 3-source integration test in Plan 03-04.

<user_constraints>
## User Constraints (from CONTEXT.md)

**No CONTEXT.md exists for this phase.** The orchestrator did not invoke `/gsd-discuss-phase` first. Constraints below are derived from project-level decisions (PROJECT.md Key Decisions, ROADMAP.md, STATE.md, CLAUDE.md, and Phase 1 + Phase 2 SUMMARY/RESEARCH locks).

### Locked Decisions (from project docs)

- **Tauri v2 + Rust** is the stack. The watcher pipeline (`WatcherCore::run_watcher`, `run_pipeline`, `CrossSourceDedup`, `SqliteStore`) is locked in Phase 1 and Phase 2. No restructuring. (PROJECT.md, Phase 1 SUMMARYs.)
- **No Steam Web API in v1.** This is a hard constraint (PROJECT.md "File watcher only" + REQUIREMENTS.md "Out of Scope"). The Steam-legit adapter MUST parse binary VDF locally and CANNOT call `GetPlayerAchievements` or any other Steam Web endpoint. Achievement-Watcher's strategy of using the binary file as an mtime trigger and then hitting the Web API is **explicitly forbidden** here.
- **Windows-only v1.** All paths can use `%APPDATA%`, `dirs::data_dir()`, `dirs::config_local_dir()` etc. without cross-platform concern.
- **File watcher only.** No process injection, no IPC into Steam client. Detection happens by `notify`-watching files and parsing them on change.
- **Local-only.** No telemetry, no cloud, no auth, no network beyond what Phase 2's schema cache already does.
- **Hobby pace, polish over speed.** Prefer reliable patterns over clever ones; lean on the verified Phase 1 patterns.
- **Goldberg/CreamAPI/SmartSteamEmu setup assistance is OUT OF SCOPE.** Adapters are **passive** — they read what is on disk; they do not install, configure, or recommend the emulator.
- **REQ DETECT-02, DETECT-03, DETECT-04 must be addressed in this phase** (REQUIREMENTS.md traceability).
- **REQ DETECT-07 (cross-source dedup) is already satisfied** in Phase 1 with a key of `(app_id, ach_api_name)` — Phase 3 must verify it generalizes to 3+ sources, not redesign it.
- **Phase 1 pipeline shape is locked** (STATE.md decisions): one shared `notify-debouncer-full` at 500ms, seed-then-attach ordering, prefix-match dispatch with multi-adapter delivery, in-memory TTL dedup (10s default) + SQLite UNIQUE INDEX.
- **Phase 2 schema cache (`SchemaCache`) and schema lookup chain are locked** (STATE.md decisions). Phase 3 adapters emit `RawUnlockEvent { app_id, ach_api_name, ... }` and the existing schema lookup chain (steam Web no-key + appcache global + goldberg meta) handles display-name resolution. The CreamAPI and SmartSteamEmu adapters do NOT bring schema; they only emit unlock events.
- **`SourceAdapter` trait is locked.** Three new variants are added to `SourceKind`: `SteamLegit`, `CreamApi`, `SmartSteamEmu`. The trait shape (5 methods: `name`, `kind`, `watch_paths`, `seed_baseline`, `on_file_changed`) does NOT change. (Phase 1 Plan 02 decision; reserved spots already noted in `sources/mod.rs:54`.)
- **Pipeline call site `lib.rs::run()` setup() closure is the integration point.** Three new adapters get appended to `let adapters = vec![goldberg_adapter, ...];` (currently a single-entry Vec — `lib.rs:201`). The existing `run_watcher` + `run_pipeline` accept this Vec without API change. (Verified by inspecting `lib.rs::run()`.)
- **Project skills:** `.claude/skills/`, `.agents/skills/` not present in this repo. No project-skill rules to honor.

### Claude's Discretion

- Choice of binary VDF reader: **hand-roll** vs **steam-vdf-parser** vs **fork keyvalues-parser**. (Recommended: hand-roll — see Don't Hand-Roll for the justification of this exception.)
- Choice of CRC32 crate: `crc32fast` (single-purpose, fast, ~0 deps) vs `crc 3.x` (general-purpose multi-polynomial). Either is acceptable.
- Choice of INI parser for CreamAPI: hand-roll (~10 LoC, Hydra's pattern) vs add `rust-ini` / `configparser` crate. Recommended: hand-roll — the file format is too simple to justify a dependency.
- Plan-decomposition shape (number of plans, allocation of work). Phase 1 used 5 plans, Phase 2 used 7 plans; Phase 3 will likely need 4–5 plans (one per adapter + path-discovery extension + integration tests).
- The order in which adapters are implemented inside the phase. Recommended: **CreamAPI first** (simplest format, INI), then SmartSteamEmu (binary but small fixed-size record), then Steam-legit (most complex, two-file dependency).
- Whether to lazily build the SmartSteamEmu CRC32→API-name reverse map at session start vs at first event arrival. Recommended: build at first event arrival per appid (lazy) — avoids paying CRC32-of-everything cost for games the user never launches.
- Whether to expose CLI overrides for the new adapters (`--override-creamapi-root`, etc.) parallel to Phase 1's `--override-goldberg-root`. Recommended yes for testability.

### Deferred Ideas (OUT OF SCOPE for Phase 3)

- **Steam Web API integration** — explicitly forbidden by REQUIREMENTS.md "Out of Scope" + PROJECT.md.
- **Process injection / DLL hooking for any adapter** — REACH-V2-01 is v2.
- **Other emulator formats** — CODEX (`%PUBLIC%\Documents\Steam\CODEX\<appid>\achievements.ini`), SKIDROW, EMPRESS, OnlineFix, RLD!, RUNE, ALI213, 3DM, Razor1911 — all OUT OF SCOPE for v1. The architecture supports them as Phase-N additions following the same pattern, but ROADMAP only covers Steam-legit + CreamAPI + SmartSteamEmu in v1.
- **Linux/Steam Deck support** — REACH-V2-02 is v2; cfg(target_os = "windows") gating is acceptable.
- **First-run UI wizard surfacing the new discovered paths** — DIST-04 is Phase 4. Phase 3 only LOGS what was discovered and persists nothing UI-visible.
- **Re-detecting newly-installed CreamAPI / SmartSteamEmu paths after Hallmark startup** — Phase 4 first-run wizard concern; Phase 3 discovers once at startup and accepts the limitation (matches Phase 1 Goldberg behaviour).
- **Schema mapping for SmartSteamEmu's CRC32 keys when no Goldberg companion file or Steam appcache schema is present** — CreamAPI / SSE games without the corresponding Steam app installed produce events with `ach_api_name = "<crc:0xDEADBEEF>"` placeholder; the schema lookup chain (Phase 2) absorbs this without crashing — flagged for Phase 4 polish.
- **Detection of legit Steam achievement unlocks for users with multiple Steam accounts where the file mtime fires for the inactive user** — handled by ALWAYS using the user-id from the filename (`UserGameStats_<userid>_<appid>.bin`), but multi-user disambiguation in the UI is Phase 4.
- **The legit-Steam-unlocked-via-Steam-overlay-only case** — when Steam writes only to its own internal cloud and the local `appcache/stats` file lags by minutes. This is a documented Steam quirk; the pragmatic answer is "we fire when the file fires." Documented as a known limitation; not a blocker.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **DETECT-02** | Real-time watcher detects unlocks from legitimate Steam (binary VDF parser of `userdata/<steamid>/<appid>/remote/`, mtime trigger via `appcache/stats`) | Empirical inspection of `C:\Program Files (x86)\Steam\appcache\stats\UserGameStats_132274694_*.bin` on dev machine — 166 real files. Format confirmed as type-tagged binary KeyValues with header `00 63 61 63 68 65 00` ("\\0cache\\0") + `crc:int32` + `PendingChanges:int32` + per-stat-slot objects containing `data:int32` and optional `AchievementTimes:object` mapping numeric bit-slot → unix seconds (int32). API name resolution is done via the per-app `UserGameStatsSchema_<appid>.bin` file in the same directory. **NOTE: Requirement description is MISLEADING — `userdata/<steamid>/<appid>/remote/` is the cloud-save directory, NOT achievement state. Phase 3 implementation uses ONLY `appcache/stats/UserGameStats_*.bin`. The `userdata` reference in REQUIREMENTS.md should be treated as informational and the implementation site corrected to `appcache/stats`.** Plan-time MUST verify this with the user OR copy this clarification into a CONTEXT.md note before starting Plan 03-01. |
| **DETECT-03** | Real-time watcher detects unlocks from CreamAPI per-appid output | Format confirmed by Hydra Launcher's CreamAPI parser (`hydralauncher/hydra/src/main/services/achievements/parse-achievement-file.ts:processCreamAPI`), Achievement-Watcher's `getAchievementsFromFile()` ini-fallback chain, and OF-Client-Launcher's `creamapi_cfg` discovery. Path: `%APPDATA%\CreamAPI\<appid>\stats\CreamAPI.Achievements.cfg`. Schema (INI): `[ACH_API_NAME]` section header + `achieved=true|false` + `unlocktime=<unix-seconds-or-microseconds>`. The `unlocktime` value can be 7 digits (treat as microseconds, multiply by 10⁶) or longer (treat as milliseconds-since-epoch). Phase 1's `earned: bool` `false→true` transition rule applies — `achieved` is the unlock signal, never `unlocktime`. |
| **DETECT-04** | Real-time watcher detects unlocks from SmartSteamEmu per-persona output | Two competing canonical formats exist: **(A)** Achievement-Watcher's `stats.bin` parser (`xan105/Achievement-Watcher/app/parser/sse.js`): 4-byte LE-int header (`expectedStatsCount`) + N×24-byte records, each: `[0..4]` CRC32-of-API-name (reversed bytes → hex string), `[8..12]` UnlockTime (Int32LE unix seconds), `[20..24]` value (achievement when 0 or 1; stat when >1). Path: `%APPDATA%\SmartSteamEmu\<appid>\stats.bin`. **(B)** Hydra's discovery (`hydralauncher/hydra/.../find-achivement-files.ts`): `%APPDATA%\SmartSteamEmu\<appid>\User\Achievements.ini` — but Hydra has NO parser registered for SmartSteamEmu in `parse-achievement-file.ts`, so this is unverified. RECOMMENDATION: Plan 03-03 implements (A) as the primary and probes both paths during discovery; if a directory contains `User\Achievements.ini` but no `stats.bin`, log a warning and treat as unsupported variant. |

REQ DETECT-07 (cross-source dedup) and DETECT-06 (debounce + content-hash) are reused from Phase 1 unchanged — no rework needed. Phase 3 adds verification tests with 3 sources active.
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Steam binary VDF reading | Rust backend (new module `vdf_binary.rs`) | — | Pure binary parsing; needs `windows`-aware byte order (always LE on x86/x64). No UI involvement. |
| Steam-legit `UserGameStats_*.bin` adapter | Rust backend (`sources::steam_legit`) | — | Implements `SourceAdapter`; reads two binary files (state + schema) per app; emits `RawUnlockEvent` like every other adapter. |
| CreamAPI INI parsing | Rust backend (`sources::cream_api`) | — | Trivial line-oriented INI; no UI involvement. |
| SmartSteamEmu `stats.bin` parsing | Rust backend (`sources::sse`) | — | Fixed-size record binary; needs CRC32 utility. |
| CRC32 reverse-lookup builder | Rust backend (in `sources::sse`) | — | Computes CRC32 over candidate API names (sourced from Phase 2's `SchemaCache` plus any optional companion file) and inverts the map. |
| Path discovery extension | Rust backend (`paths.rs`) | — | Extends existing `DiscoveredPaths` struct with three new fields; existing `discover()` runs additional enumeration; existing logging pattern reused for Success-Criterion-style observability. |
| Existing pipeline (`run_watcher`, `run_pipeline`, `CrossSourceDedup`, `SqliteStore`) | Rust backend (locked from Phase 1) | — | UNCHANGED. Phase 3 adds adapters to `Vec<Arc<dyn SourceAdapter>>` only. |
| UI / popup layer | (deferred — Phase 2 already done) | Rust+WebView | Phase 3 emits the same `RawUnlockEvent` shape Phase 2's popup queue already consumes. NO frontend work in Phase 3. |
| Schema/icon resolution for new sources | Rust backend (Phase 2's `SchemaCache`) | — | Already handles unknown-source events via the lookup chain. New adapters do NOT need to know about schema cache. |

**Tier rule:** All Phase 3 work lives entirely in `src-tauri/src/sources/` plus a helper module for binary VDF, plus extensions to `src-tauri/src/paths.rs`. The frontend, popup queue, audio dispatcher, schema cache, companion window — none change. If any plan touches `src-tauri/src/ui.rs`, `popup_queue.rs`, `schema/`, `audio.rs`, or any frontend file, it is mis-scoped.

## Standard Stack

### Core (verified versions as of 2026-05-09 via crates.io API)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tauri` | 2.11.1 | App framework (already in Cargo.toml from Phase 1) | [VERIFIED: existing Cargo.toml] No change needed in Phase 3. |
| `notify` | 8.2.0 | FS watcher (existing) | [VERIFIED: existing Cargo.toml] No change. |
| `notify-debouncer-full` | 0.7.0 | 500ms debounce + rename tracking (existing) | [VERIFIED: crates.io API 2026-05-09 — current latest stable is 0.7.0, updated 2026-05-02] No change. |
| `tokio` | 1.52.x | Async runtime (existing) | [VERIFIED: existing Cargo.toml] No change. |
| `rusqlite` | 0.39 | SQLite (existing) | [VERIFIED: existing Cargo.toml] No change. |
| `serde` + `serde_json` | 1.x | (existing) — used for any JSON-shaped CreamAPI fallback or telemetry | [VERIFIED: existing Cargo.toml] No change. |
| `walkdir` | 2.5.0 | Directory enumeration of `appcache/stats/UserGameStats_*.bin` and `%APPDATA%\CreamAPI\*` and `%APPDATA%\SmartSteamEmu\*` | [VERIFIED: existing Cargo.toml] No change — already used in Phase 1 for `steam_api*.dll` enumeration. |
| `byteorder` | 1.5.0 | LE int reading for binary VDF + SSE `stats.bin` | [VERIFIED: crates.io 2026-05-09 — 1.5.0 is current stable] **NEW dependency for Phase 3.** Standard Rust crate for endian-aware byte reading. Alternative is hand-rolling `u32::from_le_bytes` calls — acceptable but `byteorder::ReadBytesExt::read_u32::<LittleEndian>(...)` is cleaner across many sites. |
| `crc32fast` | 1.4.2 | CRC32 of API names for SSE reverse-lookup | [VERIFIED: crates.io 2026-05-09 — 1.4.2 is current stable] **NEW dependency for Phase 3.** SIMD-accelerated CRC32 (Castagnoli AND IEEE 802.3 — SSE uses IEEE/0xEDB88320). The classical `crc` 3.x crate also works but `crc32fast` is the de-facto Rust standard for this single polynomial. |
| `sha2` | 0.11.x | Per-adapter content-hash dedup (existing) | [VERIFIED: existing Cargo.toml] No change — every new adapter reuses the same per-file SHA-256 pattern Phase 1 uses. |

### Supporting (situational)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `keyvalues-parser` | 0.2.3 (existing) | Could be extended to parse the binary VDF if a downstream user prefers — but the README explicitly says **text-only**, confirmed via WebFetch on 2026-05-09. **Do NOT use for binary VDF.** | Only for any new TEXT-VDF parsing (none anticipated in Phase 3). |
| `winreg` | 0.56 (existing) | Read `HKCU\Software\Valve\Steam\Users` to enumerate Steam user IDs without globbing the `userdata/` dir | Optional. Phase 1 already reads `HKCU\Software\Valve\Steam\SteamPath`; reading `Users` subkey is an additive, low-risk extension if needed. Probably unnecessary because the userid is parseable from the filename `UserGameStats_<userid>_<appid>.bin`. |
| `steam-vdf-parser` | 0.1.1 | Possible alternative to hand-rolled binary VDF reader | **NOT RECOMMENDED.** README explicitly targets `shortcuts.vdf`, `appinfo.vdf`, `packageinfo.vdf`. UserGameStats compatibility undocumented. The simple type-tag format is small enough to hand-roll. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled binary VDF reader | `steam-vdf-parser` 0.1.1 | The crate works for shortcuts/appinfo/packageinfo but does not document UserGameStats. Adopting it without verification risks runtime parse failures on the user's actual files; verifying it against UserGameStats requires nearly the same effort as hand-rolling. The format we need is ~6 type tags. Hand-rolling is the lower-risk path for this specific file. |
| `crc32fast` | `crc` 3.2.x | `crc` is general-purpose multi-polynomial; `crc32fast` is faster (SIMD) and single-purpose. Either works; `crc32fast` is the de-facto Rust standard for IEEE 802.3 / Castagnoli. |
| Hand-rolled INI parser for CreamAPI (~12 LoC, line-oriented per Hydra) | `rust-ini` 0.21+ or `configparser` 3.x | The CreamAPI file is too simple — `[section]` + `key=value` lines, no comments to handle, no escape sequences, no quoted values. Hand-rolling matches Hydra's approach (their parser is 12 lines). Adding a crate for this is over-engineering. |
| Append three new entries to `Vec<Arc<dyn SourceAdapter>>` in `lib.rs::run()` | Refactor pipeline to a builder pattern with adapter registration | Phase 1's invariant is "adapters are a Vec; pipeline accepts Vec; no per-adapter wiring beyond constructor." Adding a builder is over-engineering for v1; matching the existing pattern keeps the diff small and reviewable. |

**Installation (single Cargo.toml block — appended to existing `[dependencies]` in `src-tauri/Cargo.toml`):**

```toml
# Phase 3 additions
byteorder = "1.5"        # LE int reading for binary VDF + SSE stats.bin
crc32fast = "1.4"        # CRC32-of-API-name for SmartSteamEmu reverse-lookup
```

**Version verification done 2026-05-09:** `notify-debouncer-full 0.7.0` confirmed current (last updated 2026-05-02), `byteorder 1.5.0` confirmed current, `crc32fast 1.4.2` confirmed current, `steam-vdf-parser 0.1.1` (last updated 2026-01-18) checked but explicitly NOT recommended (see Don't Hand-Roll #1). `keyvalues-serde 0.2.3` confirmed current — relevant only if any TEXT VDF schema work is added (not anticipated).

## Architecture Patterns

### System Architecture Diagram

```
                                                                 ┌────────────────────────────────────────────────────────────┐
[DISK]                                                           │                Phase 3 Rust process                        │
                                                                 │                                                            │
%APPDATA%\Goldberg SteamEmu Saves\<appid>\achievements.json      │  [PathDiscovery — extended in Plan 03-04]                  │
%APPDATA%\GSE Saves\<appid>\achievements.json                    │   • Phase 1 fields (steam_install, steam_libraries,        │
%PUBLIC%\Documents\Goldberg SteamEmu Saves\<appid>\...            │     goldberg_save_roots, goldberg_local_save_redirects)   │
                                                                 │   • NEW: steam_legit_user_id (registry HKCU \Users\*)      │
<SteamPath>\appcache\stats\UserGameStats_<userid>_<appid>.bin    │   • NEW: cream_api_appids (enum %APPDATA%\CreamAPI\*)      │
<SteamPath>\appcache\stats\UserGameStatsSchema_<appid>.bin       │   • NEW: sse_appids (enum %APPDATA%\SmartSteamEmu\*)       │
                                                                 │   • Extra tracing::info! per discovered category           │
%APPDATA%\CreamAPI\<appid>\stats\CreamAPI.Achievements.cfg       │            │ Vec<PathBuf> + per-source maps               │
%APPDATA%\SmartSteamEmu\<appid>\stats.bin                        │            ▼                                              │
                                                                 │  [WatcherCore (UNCHANGED FROM PHASE 1) — single shared    │
        modify event ─────────────────────────────────────────►  │     notify-debouncer-full 500ms; one prefix-match table; │
                                                                 │     dispatches to ALL matching adapters]                  │
                                                                 │            │                                              │
                                                                 │            ▼ (per-event)                                  │
                                                                 │  ┌──────────┬───────────┬───────────┬──────────────┐      │
                                                                 │  │ Goldberg │ SteamLegit│  CreamAPI │SmartSteamEmu │      │
                                                                 │  │ adapter  │  adapter  │  adapter  │   adapter    │      │
                                                                 │  │ (Phase 1)│  (NEW)    │  (NEW)    │   (NEW)      │      │
                                                                 │  └──────────┴───────────┴───────────┴──────────────┘      │
                                                                 │      │ each: filename guard → SHA-256 dedup → parse →     │
                                                                 │      │   diff vs in-memory baseline → emit RawUnlockEvent │
                                                                 │      ▼                                                    │
                                                                 │  [tokio::mpsc::Sender<RawUnlockEvent>] ─── raw_tx          │
                                                                 │      │                                                    │
                                                                 │      ▼                                                    │
                                                                 │  [run_pipeline (UNCHANGED FROM PHASE 1)]                  │
                                                                 │   1. CrossSourceDedup.is_duplicate(app_id, ach_api_name)   │
                                                                 │      → drop if seen <10s ago                              │
                                                                 │   2. SqliteStore.record_unlock (UNIQUE INDEX backstop)     │
                                                                 │   3. forward to sink_tx                                   │
                                                                 │      │                                                    │
                                                                 │      ▼                                                    │
                                                                 │  [popup_queue (Phase 2) — unchanged consumer]              │
                                                                 │   • renders the same premium popup regardless of source   │
                                                                 └────────────────────────────────────────────────────────────┘
                                                                                          │
                                                                                          ▼
                                                                                  [WebView popup overlay]
                                                                                  (REQ Success Criteria #1 + #2)

(REQ Success Criterion #3: when a user runs both legit Steam AND Goldberg+CreamAPI on the same game,
 each adapter independently emits an event for the same (app_id, ach_api_name); CrossSourceDedup
 collapses to one. Verification test in Plan 03-04 spins up 3 file-event-driven MockAdapter instances
 and asserts exactly 1 popup signal in the sink.)
```

### Recommended Project Structure

Phase 3 ADDS files; it does not move them. Layout after Phase 3:

```
src-tauri/src/
├── lib.rs                          # MODIFIED: append 3 adapters to `let adapters = vec![...]`
├── paths.rs                        # MODIFIED: add steam_legit_user_id + cream_api_appids + sse_appids fields and discovery functions
├── sources/
│   ├── mod.rs                      # MODIFIED: add SourceKind::SteamLegit / CreamApi / SmartSteamEmu variants + as_str() arms
│   ├── goldberg.rs                 # UNCHANGED (Phase 1)
│   ├── vdf_binary.rs               # NEW: hand-rolled binary VDF reader (Vdf, Obj, Value enum, parse_binary_vdf())
│   ├── steam_legit.rs              # NEW: SteamLegitAdapter (REQ DETECT-02)
│   ├── cream_api.rs                # NEW: CreamApiAdapter (REQ DETECT-03)
│   └── sse.rs                      # NEW: SseAdapter (REQ DETECT-04)
├── store/
│   ├── mod.rs                      # UNCHANGED — store accepts arbitrary `source: &str` already
│   └── migrations/
│       └── 001_initial.sql         # UNCHANGED — `unlock_history.source TEXT` accommodates new strings
└── watcher/
    └── mod.rs                      # UNCHANGED (run_watcher + run_pipeline are source-agnostic)
```

**Key invariant:** the only files modified outside `src-tauri/src/sources/` are `paths.rs` (extension), `lib.rs` (3-line edit appending to adapter Vec), and `Cargo.toml` (2 new deps). Everything else is additive.

### Pattern 1: New adapter follows GoldbergAdapter's exact shape

**What:** Each new adapter mirrors `sources::goldberg::GoldbergAdapter` — owns a constructor that takes its watch roots + (where applicable) a redirect/lookup map, holds an `Arc<RwLock<HashMap<(u64, String), bool>>>` baseline and an `Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>` last-hash map, implements all 5 `SourceAdapter` methods.

**When to use:** Every new adapter in this phase (and future v2 store adapters: Epic, GOG, Xbox).

**Example skeleton (verified against Phase 1 GoldbergAdapter pattern at `src-tauri/src/sources/goldberg.rs:56-119`):**

```rust
// src-tauri/src/sources/cream_api.rs  — illustrative only
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use super::{RawUnlockEvent, SourceAdapter, SourceKind};

pub struct CreamApiAdapter {
    /// Each entry is the `%APPDATA%\CreamAPI\<appid>\` directory whose `stats\CreamAPI.Achievements.cfg`
    /// we will watch. Resolved at startup; static for adapter lifetime (Phase 1 cached_watch_paths pattern).
    watch_roots: Vec<PathBuf>,
    /// appid lookup keyed on the parent dir — direct numeric parse of `<appid>` works here because
    /// the Cream layout always has appid as the immediate parent's name.
    baseline: Arc<RwLock<HashMap<(u64, String), bool>>>,
    last_hash: Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>,
}

impl CreamApiAdapter {
    pub fn new(watch_roots: Vec<PathBuf>) -> Self {
        let cached: Vec<PathBuf> = watch_roots.iter().filter(|p| p.exists()).cloned().collect();
        Self {
            watch_roots: cached,
            baseline: Arc::new(RwLock::new(HashMap::new())),
            last_hash: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn parse_state(text: &str) -> HashMap<String, bool> {
        // Hydra-style line parser: 12-LoC INI, [section] starts a new ach key,
        // `achieved=true` flips it to true. unlocktime is ignored for unlock decisions.
        let mut result = HashMap::new();
        let mut current: Option<String> = None;
        for line in text.lines() {
            let line = line.trim_start_matches('\u{feff}').trim();  // BOM strip per Hydra
            if line.is_empty() || line.starts_with("###") { continue; }
            if line.starts_with('[') && line.ends_with(']') {
                current = Some(line[1..line.len()-1].to_string());
            } else if let Some(name) = &current {
                if let Some((k, v)) = line.split_once('=') {
                    if k.trim().eq_ignore_ascii_case("achieved") {
                        let earned = matches!(v.trim().to_ascii_lowercase().as_str(), "true" | "1");
                        result.insert(name.clone(), earned);
                    }
                }
            }
        }
        result
    }
}

#[async_trait::async_trait]
impl SourceAdapter for CreamApiAdapter {
    fn name(&self) -> &str { "creamapi" }
    fn kind(&self) -> SourceKind { SourceKind::CreamApi }
    fn watch_paths(&self) -> Vec<PathBuf> { self.watch_roots.clone() }

    async fn seed_baseline(&self) -> anyhow::Result<()> {
        // Walk each watch root for `<appid>/stats/CreamAPI.Achievements.cfg`,
        // parse, populate baseline. Same shape as GoldbergAdapter::seed_baseline.
        // ...
        Ok(())
    }

    async fn on_file_changed(
        &self,
        path: PathBuf,
        tx: mpsc::Sender<RawUnlockEvent>,
    ) -> anyhow::Result<()> {
        // Filename guard — only `CreamAPI.Achievements.cfg` events matter.
        if path.file_name().and_then(|n| n.to_str()) != Some("CreamAPI.Achievements.cfg") {
            return Ok(());
        }
        // ... read+retry → SHA-256 dedup → parse → diff vs baseline → emit RawUnlockEvent { source: SourceKind::CreamApi, .. }
        Ok(())
    }
}
```

### Pattern 2: SteamLegitAdapter holds a per-app schema cache

**What:** Steam-legit needs TWO files per app — `UserGameStats_<userid>_<appid>.bin` (state) and `UserGameStatsSchema_<appid>.bin` (schema mapping numeric stat-slot → API name). The schema rarely changes; the state changes on every unlock. Cache the parsed schema per appid in an `Arc<RwLock<HashMap<u64, AppSchema>>>` and re-read only on schema-file mtime change.

**When to use:** Steam-legit only — no other adapter needs a two-file dependency.

**Sketch (key fields):**

```rust
pub struct SteamLegitAdapter {
    /// <SteamPath>\appcache\stats — single watched directory recursively
    appcache_stats_root: PathBuf,
    /// Steam user IDs to consider. Filename pattern: UserGameStats_<userid>_<appid>.bin.
    /// Events for unknown user IDs are ignored. Phase 3 supports the registry-detected set.
    known_user_ids: Vec<u64>,
    /// Per-app schema cache: appid → (numeric_stat_slot → ach_api_name + per-bit map)
    schema_cache: Arc<RwLock<HashMap<u64, AppSchema>>>,
    /// Standard Phase 1 baseline + content-hash patterns
    baseline: Arc<RwLock<HashMap<(u64, String), bool>>>,
    last_hash: Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>,
}

struct AppSchema {
    /// stat_slot_index (e.g. 1, 2, 3) → ach_api_name
    /// For multi-bit achievements (rare — `bits` substructure), keyed (stat, bit) tuples.
    achievements: HashMap<u32, String>,
    /// File mtime when schema was loaded; used to detect stale cache.
    loaded_mtime: SystemTime,
}
```

### Pattern 3: SmartSteamEmu CRC32→API-name reverse-lookup, lazy per appid

**What:** SSE's `stats.bin` keys are CRC32(api_name). To emit `RawUnlockEvent { ach_api_name, .. }` we need the reverse map. Build it lazily: on first `on_file_changed` event for a given appid, look up the candidate API names (sources, in priority order: Phase 2's `SchemaCache::list_for_app(app_id)`, OR a sibling Goldberg schema file `<game-dir>\steam_settings\achievements.json`, OR the empty set), CRC32 each one, build the inverse map, persist it in an `Arc<RwLock<HashMap<u64, HashMap<u32, String>>>>` for the adapter's lifetime.

**When to use:** SSE only.

**Pitfall:** Achievement-Watcher's parser notes the IEEE 802.3 CRC32 module strips a leading 0 for hashes below 0x1000 — so the SSE crc bytes might be 3 hex chars vs 4. Work around by zero-padding both sides to 8 chars when comparing. ([CITED: `xan105/Achievement-Watcher/app/parser/achievements.js` line containing `(SSE) crc module removes leading 0 when dealing with anything below 0x1000`])

### Pattern 4: Path discovery extension as additive fields, not breaking changes

**What:** Phase 1's `paths::DiscoveredPaths` is `Default + Clone + Debug` and has 4 `pub` fields. Add 3 more `pub` fields with `#[serde(default)]`-equivalent semantics (default-empty `Vec`/`Option`) and add 3 new helper functions following the `goldberg_watch_paths`/`goldberg_redirect_map` shape. Existing call sites (Phase 1 + Phase 2 `lib.rs::run()`) keep working unchanged because adding fields to a struct does not break struct-init code that was already using `..default()` semantics, and `lib.rs::run()` reads only the 4 Phase-1 fields.

**Example:**

```rust
// src-tauri/src/paths.rs — extended
#[derive(Debug, Clone, Default)]
pub struct DiscoveredPaths {
    // Phase 1 fields (UNCHANGED) ...
    pub steam_install: Option<PathBuf>,
    pub steam_libraries: Vec<PathBuf>,
    pub goldberg_save_roots: Vec<PathBuf>,
    pub goldberg_local_save_redirects: Vec<GoldbergRedirect>,
    // Phase 3 additions:
    pub steam_legit_appcache_stats: Option<PathBuf>,   // <SteamPath>\appcache\stats
    pub steam_legit_user_ids: Vec<u64>,                 // parsed from registry HKCU\Software\Valve\Steam\Users
    pub cream_api_appid_dirs: Vec<PathBuf>,             // %APPDATA%\CreamAPI\<appid>\ — one entry per existing appid
    pub sse_appid_dirs: Vec<PathBuf>,                   // %APPDATA%\SmartSteamEmu\<appid>\
}

pub fn steam_legit_watch_paths(d: &DiscoveredPaths) -> Vec<PathBuf> {
    // returns vec![d.steam_legit_appcache_stats] if Some, else empty
    d.steam_legit_appcache_stats.iter().cloned().collect()
}

pub fn cream_api_watch_paths(d: &DiscoveredPaths) -> Vec<PathBuf> {
    d.cream_api_appid_dirs.clone()
}

pub fn sse_watch_paths(d: &DiscoveredPaths) -> Vec<PathBuf> {
    d.sse_appid_dirs.clone()
}
```

### Anti-Patterns to Avoid

- **Calling Steam Web API as a fallback when binary VDF parsing fails.** Explicitly forbidden by project constraint. If parsing fails, log a warning and skip the file (adapter continues, other adapters keep working). Do NOT silently degrade to web fetching.
- **Trusting the `unlocktime` field as the unlock signal in CreamAPI** (mirrors Phase 1 PITFALL #15 for Goldberg). The `achieved=true` boolean transition is the only reliable signal. CreamAPI sometimes writes `unlocktime=0` for unknown timestamps, and 7-digit microsecond-encoded values mean "real timestamp" — neither pattern is suitable for unlock detection.
- **Using `userdata/<steamid>/<appid>/remote/` for legit Steam achievement state.** That directory is for Steam Cloud save files (`*.sav`, game-specific). It has nothing to do with achievements. The state is in `<SteamPath>\appcache\stats\UserGameStats_<userid>_<appid>.bin`. REQUIREMENTS.md DETECT-02's wording is misleading; the planner must clarify with the user OR copy this clarification verbatim into a CONTEXT.md.
- **Hardcoding stat-slot → API-name maps for legit Steam.** Stat slots are per-app; they DIFFER between games. Always go through `UserGameStatsSchema_<appid>.bin`. Do NOT assume "stat slot 1 = first achievement" — many games use stat slot 1 as a non-achievement stat (e.g. play count) and reserve a `bits` substructure for the per-achievement bits.
- **Adding overlapping watch paths.** Phase 1's WatcherCore now dispatches to ALL matching adapters (BL-03 fix in Plan 01-04 + WR-09 mitigation), so overlapping paths cause duplicate events that must be collapsed by `CrossSourceDedup`. To avoid relying on that, ensure new adapters' watch roots do NOT prefix-match Goldberg roots — verified for the planned paths: Goldberg uses `%APPDATA%\Goldberg SteamEmu Saves` / `%APPDATA%\GSE Saves` / `%PUBLIC%\Documents\Goldberg SteamEmu Saves`, and Phase 3 adds `%APPDATA%\CreamAPI`, `%APPDATA%\SmartSteamEmu`, `<SteamPath>\appcache\stats` — no overlaps. **Document inline in adapter constructors.**
- **Spawning a per-adapter background sweep task.** The Phase 1 dedup invariant assumes ONE consumer sweeps the dedup map (`run_pipeline`). Adding more sweepers creates lock contention and timing edge cases.
- **Storing `SourceKind::CreamApi as u8 = 2`-style numeric tags in SQLite.** Phase 1 `SourceKind::as_str()` is the contract — uses lowercase strings like `"goldberg"`, persisted as TEXT. New variants follow same convention: `"steam_legit"`, `"cream_api"`, `"smartsteamemu"` (lowercase, stable, schema-migration-safe).
- **Re-reading the `UserGameStatsSchema_<appid>.bin` file on every state-change event.** It rarely changes (only when the developer ships a content update with new achievements). Cache it by mtime; only re-read when mtime advances.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Steam binary VDF reader (the FULL appinfo/packageinfo format with string tables, multi-section, etc.) | Hand-rolled multi-format parser | (None of the existing crates fully cover UserGameStats — see exception below) | The full binary VDF spec is large; if we ever need appinfo.vdf parsing, reach for `steam-vdf-parser`. **For UserGameStats specifically, hand-roll a 6-tag minimal reader (exception — see "Justified Hand-Roll" below).** |
| File system watching with debounce | per-adapter `notify` instances | Single shared `notify-debouncer-full` 0.7 (already done in Phase 1's `run_watcher`) | Locked from Phase 1 — adding adapters means adding entries to the existing `Vec<Arc<dyn SourceAdapter>>`, not new watchers. |
| Cross-source dedup | Per-event `HashMap` checks scattered through adapters | Phase 1's `CrossSourceDedup` (`watcher/dedup.rs`) | Already correct for N adapters by `(app_id, ach_api_name)` keying. |
| SQLite UNIQUE constraint for belt-and-suspenders dedup | Custom de-dup query | Existing `idx_unlock_dedup ON unlock_history(app_id, ach_api_name, session_id)` | Locked from Phase 1; new adapters insert via the same `record_unlock` API. |
| Content-hash file equality | Per-adapter byte-equality check | `sha2::Sha256` per-file hash (existing pattern in `goldberg.rs:250-258`) | Reuse pattern; every new adapter's `on_file_changed` follows identical layout. |
| Windows ERROR_SHARING_VIOLATION retry | Hand-rolled retry loop in each adapter | Lift `read_with_retry` from `goldberg.rs` into `sources::shared` or duplicate the 5-line function | Phase 1's `read_with_retry` is private to `goldberg.rs` (free function in same module); Phase 3 should EITHER move it to `sources/util.rs` and re-export, OR each adapter copies the 5-line helper. Recommended: extract once. |
| CRC32 (IEEE 802.3 polynomial 0xEDB88320) | Hand-rolled CRC32 implementation | `crc32fast` 1.4.2 | SIMD-accelerated, single-purpose, ~zero deps. Hand-rolling is correct but pointless when a 1-dep crate exists. |
| Endian-aware byte reading | Manual `u32::from_le_bytes(arr.try_into()?)` everywhere | `byteorder::ReadBytesExt` | Optional. Hand-rolling is acceptable; `byteorder` reads fewer lines and produces cleaner errors. |

### Justified Hand-Rolls (cases where building it ourselves IS the right call)

| Hand-Roll | Why Justified |
|-----------|---------------|
| **Binary VDF reader for UserGameStats**, ~250 LoC | No production-ready Rust crate documents UserGameStats compatibility. `steam-vdf-parser 0.1.1` (last updated 2026-01-18) targets shortcuts/appinfo/packageinfo; vetting it requires nearly the same effort as hand-rolling a focused parser. The format we actually need is small: 6 type tags (0x00 Object, 0x01 String, 0x02 Int32, 0x03 Float, 0x07 UInt64, 0x08 ObjectEnd) — `0x05 WString`, `0x04 Pointer`, `0x06 Color` are not used by UserGameStats per empirical inspection. The reader produces a recursive `Value::Object(HashMap<String, Value>)` / `Value::Int32(i32)` / `Value::Uint64(u64)` enum tree fitting the existing `keyvalues_parser::Vdf` consumption pattern in `paths.rs`. |
| **CreamAPI INI parser**, ~12 LoC | Hydra Launcher's parser is 12 lines and handles the file's full grammar. `rust-ini 0.21` adds 3 transitive deps for what is a simpler grammar than `/etc/passwd`. |
| **SSE 24-byte record reader**, ~15 LoC | Achievement-Watcher's `sse.js` parser is 15 lines and the format is fully known: header(4) + N×24 records, fixed offsets within each record. No crate exists for this format. |

**Key insight:** Phase 3 is **mostly** a "wire up new adapter implementations behind the existing trait" exercise. Three small hand-rolls (binary VDF reader, INI parser, SSE record reader) are the only NEW load-bearing parsers; everything else reuses Phase 1+2 infrastructure.

## Runtime State Inventory

> Phase 3 ADDS adapters. It does not rename, refactor, or migrate any existing data. This section is included for completeness with explicit "None — verified" answers per the rename/refactor checklist.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — verified by `Read` of `src-tauri/src/store/migrations/001_initial.sql`. The `unlock_history` table's `source TEXT` column accepts arbitrary lowercase strings; new variants `"steam_legit"`, `"cream_api"`, `"smartsteamemu"` insert without schema change. The `idx_unlock_dedup` UNIQUE INDEX keys on `(app_id, ach_api_name, session_id)` — source-agnostic, so cross-source dedup works at the DB level for any number of sources without migration. | None — additive only |
| Live service config | None — Hallmark has no external services. The new adapters READ from third-party emulator output paths but do not own any live service config. | None |
| OS-registered state | Phase 4 will add `HKCU\...\Run` entry; Phase 3 adds NONE. Phase 3 only READS the registry (`HKCU\Software\Valve\Steam\Users` for Steam-legit user-id enumeration — already permitted by existing `winreg` dep). | None new in Phase 3 |
| Secrets/env vars | None — no API keys, no auth. Test-time CLI overrides (`HALLMARK_GOLDBERG_ROOT_OVERRIDE`) extended with parallel env vars for new adapters (`HALLMARK_CREAMAPI_ROOT_OVERRIDE`, `HALLMARK_SSE_ROOT_OVERRIDE`, `HALLMARK_STEAM_APPCACHE_OVERRIDE`) — these are TEST conveniences, not production secrets. | None new |
| Build artifacts / installed packages | The two new Cargo deps (`byteorder`, `crc32fast`) are added to `src-tauri/Cargo.toml`. Existing `target/` directory is rebuilt by Cargo; no stale-artifact concern. | None — `cargo build` reflects Cargo.toml |

**Verified by:** Read of `src-tauri/Cargo.toml`, `src-tauri/src/store/migrations/001_initial.sql`, `src-tauri/src/sources/mod.rs`, and `src-tauri/src/lib.rs` on 2026-05-09.

## Common Pitfalls

### Pitfall 1: Confusing `userdata/<steamid>/<appid>/remote/` with achievement state

**What goes wrong:** REQUIREMENTS.md DETECT-02 mentions `userdata/<steamid>/<appid>/remote/` — a planner who reads only the requirement might point the Steam-legit adapter at the wrong directory and watch files that have nothing to do with achievements.

**Why it happens:** `userdata/<steamid>/<appid>/remote/` is for Steam Cloud save files (game-specific blobs). The actual achievement state for legit Steam is at `<SteamPath>\appcache\stats\UserGameStats_<userid>_<appid>.bin`. Achievement-Watcher confirms this — `app/parser/steam.js:scanLegit()` reads `path.join(steamPath, "appcache/stats")` and globs for `UserGameStats_*([0-9])_*([0-9]).bin`.

**How to avoid:** Plan 03-01 (Steam-legit adapter) MUST point at `<SteamPath>\appcache\stats` and watch `UserGameStats_<userid>_<appid>.bin` files. The `userdata` reference in REQUIREMENTS.md should be flagged in a CONTEXT.md note or the planner should propose a one-line REQUIREMENTS.md fix.

**Warning signs:** Test fails because no events fire even after a known-good achievement unlock; logs show watcher attached to `userdata/.../remote/` instead of `appcache/stats`.

### Pitfall 2: Binary VDF type-tag misalignment from incomplete spec

**What goes wrong:** Hand-rolled binary VDF reader handles type tag 0x02 (Int32) but not 0x07 (UInt64) — and a single `UInt64` field elsewhere in a real file (e.g. Steam build counter) causes the parser to desync, producing garbage achievement state.

**Why it happens:** [CITED: `mexus/steam-vdf-parser` README via WebFetch 2026-05-09] Binary VDF type tags include 0x00 (Object), 0x01 (String, NUL-terminated UTF-8), 0x02 (Int32), 0x03 (Float), 0x04 (Pointer, 4 bytes), 0x05 (WString, UTF-16LE), 0x06 (Color, 4 bytes RGBA), 0x07 (UInt64), 0x08 (ObjectEnd). Empirical inspection of `UserGameStats_132274694_546560.bin` shows tags `0x00`, `0x02`, `0x08` only; `UserGameStats_132274694_1237970.bin` confirms same set. Schema files `UserGameStatsSchema_*.bin` add `0x01` (string for the achievement API name and localized labels). 0x07 (UInt64) is not observed in achievement files but is documented to appear in appinfo — defensive parsers should at least skip it correctly.

**How to avoid:** Implement all 8 type tags in `vdf_binary.rs`, even those we don't expect. For unknown type tags, log a structured warn and SKIP the entry rather than panic — the parser must be tolerant to format additions in future Steam client updates.

**Warning signs:** Parse failures for some games but not others; specifically games with rich stats (vs games with only achievements).

### Pitfall 3: SmartSteamEmu CRC32 leading-zero stripping

**What goes wrong:** SSE's `stats.bin` records contain 4 bytes (reversed) for the CRC32 of the API name. JavaScript's `crc` module (per Achievement-Watcher's parser) STRIPS leading zeros for hashes < 0x1000 — but Rust's `crc32fast` does NOT. If the implementation compares hex strings without zero-padding, lookups for low-value CRCs (1 in ~16 million chance, but happens) silently fail.

**Why it happens:** [CITED: `xan105/Achievement-Watcher/app/parser/achievements.js` — comment "(SSE) crc module removes leading 0 when dealing with anything below 0x1000"]

**How to avoid:** When building the reverse-lookup, ALWAYS zero-pad the hex string to exactly 8 characters: `format!("{:08x}", crc)`. When extracting from the file's 4-byte record, also format as `{:08x}` after `u32::from_be_bytes` (the bytes are reversed, so big-endian read of the reversed slice gives the natural CRC).

**Warning signs:** Some achievements consistently fail to fire; lookup returns `None` for a small percentage of CRCs.

### Pitfall 4: CreamAPI `unlocktime` length-based unit ambiguity

**What goes wrong:** CreamAPI INI files store `unlocktime` as a string. Sometimes it's a unix-seconds value (10 digits), sometimes 7 digits encoding microseconds (`* 1000 * 1000` in Hydra/Achievement-Watcher), sometimes 13 digits (milliseconds-since-epoch). Treating all three as the same unit produces wildly wrong timestamps, which can affect telemetry but NOT unlock detection.

**Why it happens:** [CITED: `hydralauncher/hydra/.../parse-achievement-file.ts:processCreamAPI` and `xan105/Achievement-Watcher/app/parser/steam.js` lines containing `result[i].unlocktime.length === 7`]

**How to avoid:** Phase 1's rule applies — `achieved` boolean transition is the unlock signal; `unlocktime` is captured for telemetry only. The adapter writes `RawUnlockEvent { timestamp: 0, .. }` if unlocktime is unparseable or in an unknown format; downstream consumers (popup queue) already tolerate `timestamp == 0` per Phase 1 PITFALLS.md #15.

**Warning signs:** Telemetry shows wildly old or future timestamps for CreamAPI events; not user-visible but visible in `unlock_history.unlocked_at` column.

### Pitfall 5: Multi-user Steam machines firing for the wrong account

**What goes wrong:** A user has multiple Steam accounts on the same Windows profile (developer machines, family-share, account swaps). When account A unlocks an achievement, account B's `UserGameStats_<userB>_<appid>.bin` may also be touched by Steam's sync mechanism, or both files' mtimes update. The naive watcher fires on the wrong account's file.

**Why it happens:** Steam's `appcache/stats` is shared across all logged-in accounts on the same machine. The filename embeds the user ID so we know whose unlock it is, but we must NOT emit events for the inactive account.

**How to avoid:** At startup, read `HKCU\Software\Valve\Steam\Users` (registry) to enumerate all known users. The current "active" user can be derived from `HKCU\Software\Valve\Steam\AutoLoginUser` (string username) and the `loginusers.vdf` file in `<SteamPath>\config\loginusers.vdf` (which maps username → SteamID64). Phase 3 v1 handles ONLY the AutoLogin user — multi-user disambiguation in the popup ("achievement unlocked for [accountname]") is Phase 4 polish. Events for non-AutoLogin user files are silently dropped at adapter level with a `tracing::debug!` log.

**Warning signs:** Phantom popups for achievements the user never earned (because the OTHER account on the same machine earned them).

### Pitfall 6: SteamLegit baseline-vs-event race when Steam launches a game during Hallmark startup

**What goes wrong:** Hallmark starts. `seed_baseline()` reads all `UserGameStats_*.bin` files. Steam, simultaneously, processes a deferred achievement unlock from when the user was offline. The state file is REWRITTEN between baseline read and watcher attach — but the change happens DURING the `seed_baseline → new_debouncer` gap. The result: an unlock whose state was "earned" at baseline read AND "earned" after attach — silent, no event. (Race with very narrow window.)

**Why it happens:** Steam writes `UserGameStats_<userid>_<appid>.bin` non-atomically (open-write-close, not write-tmp-rename). The Phase 1 `notify-debouncer-full + 500ms` collapses bursts but cannot resurrect events that fired before the watcher was attached. This is the same problem REQ DETECT-05 was designed to handle for Goldberg, but Steam's write semantics are different from Goldberg's (Goldberg writes a complete file each time; Steam may write incrementally).

**How to avoid:** The Phase 1 invariant ("seed before attach, accept that pre-startup unlocks are absorbed") applies UNCHANGED here. The pragmatic answer for v1: any achievement earned during the millisecond seed→attach gap is silently absorbed. This is consistent with REQ DETECT-05's "no historic spam" priority, and matches Goldberg behaviour. Document inline in `SteamLegitAdapter::seed_baseline` and accept the limitation.

**Warning signs:** Extremely rare reports of "I unlocked X but no popup" for achievements unlocked precisely as Hallmark started. Not actionable.

### Pitfall 7: SmartSteamEmu's two-format ambiguity (Achievement-Watcher's `stats.bin` vs Hydra's `User\Achievements.ini`)

**What goes wrong:** Achievement-Watcher canonicalizes `%APPDATA%\SmartSteamEmu\<appid>\stats.bin`; Hydra canonicalizes `%APPDATA%\SmartSteamEmu\<appid>\User\Achievements.ini`. Implementing only one path misses installations that use the other.

**Why it happens:** [CITED: both repos via `gh search code` 2026-05-09] SSE has had at least two save formats over its lifetime; both are in current use depending on the SSE version installed.

**How to avoid:** Plan 03-03 (SSE adapter) probes BOTH paths during discovery. If `stats.bin` exists, use the Achievement-Watcher binary parser. If `stats.bin` is absent but `User\Achievements.ini` exists, log a `warn!` and treat as unsupported variant for v1 (defer INI variant to Phase 4 if user reports surface). v1 ships ONE format (`stats.bin`) and documents the other clearly.

**Warning signs:** Some SSE-using games never produce events; user log shows "found %APPDATA%\SmartSteamEmu\1234\User\Achievements.ini but no stats.bin — variant not supported in v1."

### Pitfall 8: Schema file `UserGameStatsSchema_<appid>.bin` may be MISSING

**What goes wrong:** A Steam game has a `UserGameStats_<userid>_<appid>.bin` (state) but no `UserGameStatsSchema_<appid>.bin` (schema). The state file changes; we cannot map the numeric stat slot to an API name; we cannot emit a meaningful `RawUnlockEvent`.

**Why it happens:** Schema files are downloaded by Steam on first achievement-stats query for the app. A user who has the game installed but has never opened it (or has never had achievements queried) may have the state file but not the schema file. This is rare but happens with games installed via Steam family-share.

**How to avoid:** In `on_file_changed`, if the schema cache miss occurs and the schema file does not exist on disk, emit `RawUnlockEvent { ach_api_name: format!("steam_stat_{}", stat_slot), .. }` with a placeholder API name keyed on the stat slot. The Phase 2 `SchemaCache` lookup chain will treat this as an unknown achievement (display fallback: "Achievement [stat_slot]"), and the popup will fire with degraded but non-broken UI. Log structured warn so users see "schema file missing for app 12345 — using fallback display."

**Warning signs:** Popups display "Achievement 7" instead of "Hit and Run" for a small minority of games.

### Pitfall 9: Watch path overlap between Steam-legit (`<SteamPath>\appcache\stats`) and any other adapter

**What goes wrong:** A future adapter (in v2) might watch a sibling path under `<SteamPath>` and overlap. Phase 1's WatcherCore logs an `error!` for overlap and dispatches to all matching adapters; downstream dedup catches it.

**Why it happens:** Phase 1 BL-03/WR-09 fix.

**How to avoid:** Document in each adapter's module-level docs the exact paths it claims, and any sibling paths it intentionally avoids. Plan 03-04's verification test asserts no overlap warnings appear at startup with the 4-adapter configuration.

**Warning signs:** `tracing::error!` line at startup: "adapter watch paths overlap; events may be routed to multiple adapters."

## Code Examples

### Binary VDF reader skeleton (verified against empirical inspection of UserGameStats_132274694_546560.bin)

```rust
// src-tauri/src/sources/vdf_binary.rs — reference implementation outline
// [VERIFIED: type tags match xxd output of UserGameStats_132274694_546560.bin and UserGameStatsSchema_546560.bin]
// [CITED: type-tag set documented at github.com/mexus/steam-vdf-parser README]

use std::collections::HashMap;
use std::io::{Cursor, Read};
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug, Clone)]
pub enum Value {
    Object(HashMap<String, Value>),
    String(String),
    Int32(i32),
    Float(f32),
    UInt64(u64),
    // 0x04 Pointer, 0x05 WString, 0x06 Color: parse + skip; rare in achievement files
}

#[derive(Debug, Clone)]
pub struct Vdf {
    pub root_key: String,
    pub root: Value,
}

pub fn parse_binary_vdf(bytes: &[u8]) -> anyhow::Result<Vdf> {
    let mut cursor = Cursor::new(bytes);
    let root_key = read_cstr(&mut cursor)?;
    let root = read_object_body(&mut cursor)?;
    Ok(Vdf { root_key, root })
}

fn read_object_body<R: Read>(r: &mut R) -> anyhow::Result<Value> {
    let mut map = HashMap::new();
    loop {
        let mut tag = [0u8; 1];
        if r.read(&mut tag)? == 0 { break; } // EOF — top-level object end
        match tag[0] {
            0x00 => {
                let key = read_cstr(r)?;
                let val = read_object_body(r)?;
                map.insert(key, val);
            }
            0x01 => {
                let key = read_cstr(r)?;
                let val = read_cstr(r)?;
                map.insert(key, Value::String(val));
            }
            0x02 => {
                let key = read_cstr(r)?;
                let v = r.read_i32::<LittleEndian>()?;
                map.insert(key, Value::Int32(v));
            }
            0x03 => {
                let key = read_cstr(r)?;
                let v = r.read_f32::<LittleEndian>()?;
                map.insert(key, Value::Float(v));
            }
            0x07 => {
                let key = read_cstr(r)?;
                let v = r.read_u64::<LittleEndian>()?;
                map.insert(key, Value::UInt64(v));
            }
            0x08 => return Ok(Value::Object(map)),  // ObjectEnd
            other => anyhow::bail!("unknown VDF type tag 0x{:02x}", other),
        }
    }
    Ok(Value::Object(map))
}

fn read_cstr<R: Read>(r: &mut R) -> anyhow::Result<String> {
    let mut buf = Vec::with_capacity(32);
    let mut byte = [0u8; 1];
    loop {
        r.read_exact(&mut byte)?;
        if byte[0] == 0 { break; }
        buf.push(byte[0]);
    }
    Ok(String::from_utf8(buf)?)
}
```

This produces a recursive `Value` tree. Higher-level code in `steam_legit.rs` walks the tree to extract `cache.<stat_slot>.AchievementTimes.<bit_slot>` mapping (state file) and `<appid>.stats.<stat_slot>.bits.<bit_slot>.name` mapping (schema file).

### CreamAPI INI parser (verified against Hydra's parser shape)

```rust
// src-tauri/src/sources/cream_api.rs — illustrative
// [CITED: hydralauncher/hydra/src/main/services/achievements/parse-achievement-file.ts:iniParse]

fn parse_creamapi_state(text: &str) -> std::collections::HashMap<String, bool> {
    let mut result = std::collections::HashMap::new();
    let lines: Vec<&str> = text.trim_start_matches('\u{feff}').lines().collect();
    let mut current: Option<String> = None;
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with("###") { continue; }
        if line.starts_with('[') && line.ends_with(']') {
            current = Some(line[1..line.len()-1].to_string());
        } else if let Some(name) = &current {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim().eq_ignore_ascii_case("achieved") {
                    let earned = matches!(
                        v.trim().to_ascii_lowercase().as_str(),
                        "true" | "1"
                    );
                    result.insert(name.clone(), earned);
                }
            }
        }
    }
    result
}
```

### SmartSteamEmu `stats.bin` parser (verified against Achievement-Watcher's sse.js)

```rust
// src-tauri/src/sources/sse.rs — illustrative
// [CITED: xan105/Achievement-Watcher/app/parser/sse.js — 24-byte record format]

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

#[derive(Debug, Clone)]
pub struct SseRecord {
    pub crc32_hex: String,    // zero-padded 8-char hex per Pitfall #3
    pub achieved: bool,
    pub unlock_time: u32,
}

pub fn parse_sse_stats(bytes: &[u8]) -> anyhow::Result<Vec<SseRecord>> {
    let mut cursor = Cursor::new(bytes);
    let count = cursor.read_i32::<LittleEndian>()?;
    let count = if count < 0 { 0 } else { count as usize };
    let mut out = Vec::with_capacity(count);
    let mut record = [0u8; 24];
    for _ in 0..count {
        if cursor.read_exact(&mut record).is_err() { break; }
        let crc_bytes = [record[3], record[2], record[1], record[0]]; // reversed → big-endian read
        let crc = u32::from_be_bytes(crc_bytes);
        let unlock_time = u32::from_le_bytes([record[8], record[9], record[10], record[11]]);
        let value = i32::from_le_bytes([record[20], record[21], record[22], record[23]]);
        if value > 1 { continue; } // skip stats > 1; only achievements (0/1) wanted
        out.push(SseRecord {
            crc32_hex: format!("{:08x}", crc),
            achieved: value == 1,
            unlock_time,
        });
    }
    Ok(out)
}
```

### Adapter wiring in `lib.rs::run()` (3-line addition)

```rust
// In lib.rs::run() setup() closure, replace the existing
//     let goldberg_adapter: Arc<dyn SourceAdapter> = ...;
//     let adapters = vec![goldberg_adapter];
// with:
let goldberg_adapter: std::sync::Arc<dyn sources::SourceAdapter> =
    std::sync::Arc::new(sources::goldberg::GoldbergAdapter::new(goldberg_paths.clone(), goldberg_map.clone()));
let steam_legit_adapter: std::sync::Arc<dyn sources::SourceAdapter> =
    std::sync::Arc::new(sources::steam_legit::SteamLegitAdapter::new(
        paths::steam_legit_watch_paths(&discovery),
        discovery.steam_legit_user_ids.clone(),
    ));
let cream_api_adapter: std::sync::Arc<dyn sources::SourceAdapter> =
    std::sync::Arc::new(sources::cream_api::CreamApiAdapter::new(
        paths::cream_api_watch_paths(&discovery),
    ));
let sse_adapter: std::sync::Arc<dyn sources::SourceAdapter> =
    std::sync::Arc::new(sources::sse::SseAdapter::new(
        paths::sse_watch_paths(&discovery),
    ));
let adapters = vec![goldberg_adapter, steam_legit_adapter, cream_api_adapter, sse_adapter];
```

That is the complete integration delta — `run_watcher`, `run_pipeline`, `CrossSourceDedup`, `SqliteStore`, all unchanged.

## State of the Art

| Old Approach (Phase 1, Goldberg only) | Current Approach (Phase 3) | When Changed | Impact |
|----------------|------------------|--------------|--------|
| `Vec<Arc<dyn SourceAdapter>>` of length 1 | `Vec<Arc<dyn SourceAdapter>>` of length 4 | This phase | None — locked-in pipeline accepts arbitrary length. |
| `SourceKind { Goldberg }` | `SourceKind { Goldberg, SteamLegit, CreamApi, SmartSteamEmu }` | This phase | Add 3 enum variants + 3 `as_str()` arms; trivial. |
| `DiscoveredPaths` 4 fields | `DiscoveredPaths` 7 fields (Phase 1's 4 + steam-legit + cream-api + sse) | This phase | Additive struct extension; existing call sites unaffected. |
| Achievement-Watcher's mtime-trigger + Steam Web API for legit Steam | Direct binary VDF parsing locally (no Web API) | This phase | More implementation work, but matches v1 "no Web API" constraint and gives offline support. |

**No deprecations** — this phase is purely additive.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | DETECT-02's `userdata/<steamid>/<appid>/remote/` reference is misleading; the actual path is `appcache/stats/UserGameStats_<userid>_<appid>.bin` | Phase Requirements + Pitfall #1 | If the user actually intends `userdata/.../remote/`, Phase 3 misses the requirement entirely. **Recommended:** Plan 03-00 spike confirms with user before Plan 03-01 starts; OR copy the clarification into a CONTEXT.md as a locked decision. **Confidence in correction:** HIGH — empirical inspection of 166 real UserGameStats files on dev machine, Achievement-Watcher canonical confirmation, Hydra confirmation. |
| A2 | Binary VDF type tags 0x00, 0x01, 0x02, 0x03, 0x07, 0x08 are sufficient for both UserGameStats and UserGameStatsSchema files | Standard Stack — Justified Hand-Rolls + Code Examples | If a real Steam install has a UserGameStatsSchema file using 0x05 (WString) for localized strings (UTF-16), parser fails. **Mitigation:** parser logs unknown tag and skips that entry rather than panic. **Confidence:** MEDIUM — empirical inspection covered 3 schema files and 5 state files; not exhaustive across all 166 local UserGameStats files; not exhaustive across the Steam catalog. Plan 03-01 should write a fixture test that loads the 5 inspected files and verifies parse success. |
| A3 | The CreamAPI INI section header is the achievement API name (matches Hydra and Achievement-Watcher behaviour) | Phase Requirements + Code Examples | If section headers are something else (e.g. lowercased/normalized), the `RawUnlockEvent.ach_api_name` will mismatch what Phase 2's SchemaCache expects. **Mitigation:** Phase 2 SchemaCache tolerates case mismatch via `eq_ignore_ascii_case`. **Confidence:** MEDIUM — confirmed by both canonical OSS parsers but not empirically validated on a real CreamAPI install (no CreamAPI install exists on dev machine). Plan 03-02 should request a CreamAPI fixture or a user-provided sample. |
| A4 | SmartSteamEmu's `stats.bin` 24-byte record format is per Achievement-Watcher's sse.js | Phase Requirements + Code Examples | If newer SSE versions changed the record size, parser produces nonsense. **Mitigation:** Hydra references a different path (`User\Achievements.ini`) suggesting newer SSE may have changed format entirely. Plan 03-03 probes for both `stats.bin` and `User\Achievements.ini` and ships only `stats.bin` in v1. **Confidence:** MEDIUM — single canonical source (Achievement-Watcher), no other reference, no SSE install on dev machine to verify. |
| A5 | Phase 1 `CrossSourceDedup` keyed on `(app_id, ach_api_name)` correctly generalizes to N sources | Pattern 1 (architecture) + Cross-source dedup | If 3+ identical events arrive within TTL, all but first are dropped — desired. If 3 different `ach_api_name` values for the same app_id arrive, all 3 pass — desired. The TTL window of 10s is generous; sub-second simultaneity is the real-world case. **Confidence:** HIGH — re-read of `watcher/dedup.rs` (Phase 1 source code) shows the keying is source-agnostic. Plan 03-04's verification test asserts this with 3 file-event-driven `MockAdapter`s. |
| A6 | The `SchemaCache` lookup chain in Phase 2 absorbs unknown `ach_api_name` values (e.g. `"steam_stat_7"` placeholder when schema file is missing) without crashing | Pitfall #8 | If Phase 2's lookup chain panics on an unknown API name, all of Phase 3 is broken. **Mitigation:** Phase 2 SUMMARY notes "tolerant fallback" but does not exhaustively test placeholder values. Plan 03-01 should write an integration test that fires a `RawUnlockEvent { ach_api_name: "steam_stat_7", .. }` and asserts the popup queue handles it. **Confidence:** MEDIUM — Phase 2 was designed for unknown values but the placeholder pattern is new in Phase 3. |
| A7 | Hand-rolled binary VDF reader is 250 LoC and has correctness parity with the reference shape produced by `keyvalues_parser::Vdf` | Justified Hand-Rolls | If the implementation is subtly wrong (e.g. mishandles nested objects beyond depth 3), real Steam files parse but extracted values are subtly off. **Mitigation:** Plan 03-01 writes a fixture-based round-trip test using 3 known local `.bin` files. **Confidence:** MEDIUM. |
| A8 | `byteorder` 1.5.0 and `crc32fast` 1.4.2 are stable and will compile cleanly with the existing `tokio` 1.52 + `notify` 8.2 stack | Standard Stack | If a transitive-dep conflict arises (rare for these well-isolated crates), Plan 03-01 must choose hand-rolled `from_le_bytes` instead. **Confidence:** HIGH — both crates have minimal deps; Cargo's resolver handles them trivially. |

**Empirically VERIFIED (not assumed):** the 8 type tags observed in `UserGameStats_132274694_546560.bin` and `UserGameStatsSchema_546560.bin` (0x00, 0x01, 0x02, 0x08 directly observed; 0x03, 0x07 documented but not in this sample). The `cache → crc → PendingChanges → <stat_slot> → data` skeleton structure of the state file. The 166 real UserGameStats files in `<SteamPath>\appcache\stats` with mtimes ranging 2025-04-09 to 2026-05-03 (active use). The `appcache/stats/UserGameStatsSchema_*.bin` schema file presence (1:1 with state files for installed games). Phase 1's `CrossSourceDedup` keying.

## Open Questions

1. **DETECT-02 path discrepancy with REQUIREMENTS.md**
   - What we know: `appcache/stats/UserGameStats_*.bin` is the canonical achievement state path (empirical + canonical OSS).
   - What's unclear: Whether the ROADMAP author intended `userdata/<steamid>/<appid>/remote/` for a specific reason (e.g. cloud-saved screenshots, `vrachievements`, extension hooks).
   - Recommendation: Plan 03-00 spike (or `/gsd-discuss-phase` re-invocation) MUST clarify with user. Default action: implement against `appcache/stats` and propose a one-line REQUIREMENTS.md correction.

2. **SSE 2nd format (`User\Achievements.ini`) — ship in v1 or defer?**
   - What we know: Achievement-Watcher uses `stats.bin`, Hydra discovery references `User\Achievements.ini`. SSE has at least 2 formats.
   - What's unclear: Real-world prevalence; is `stats.bin` enough to cover most modern SSE installs?
   - Recommendation: Ship `stats.bin` in v1. Log "found `User\Achievements.ini` but no `stats.bin`" as warn. Defer the INI variant to Phase 4 if user reports come in. Document in adapter's module-level `//!` block.

3. **Schema file fallback when `UserGameStatsSchema_<appid>.bin` is absent**
   - What we know: Phase 2 SchemaCache has a tolerant fallback. Pitfall #8 documents the placeholder pattern (`"steam_stat_7"`).
   - What's unclear: Whether the popup queue's tier-based animation (POPUP-06) produces a sensible visual when the API name is a placeholder.
   - Recommendation: Plan 03-01 verifies popup renders gracefully (default tier, no rarity badge, fallback display name). If broken, escalate as a Phase 2 hardening task.

4. **Multi-Steam-user disambiguation**
   - What we know: Multiple users on the same machine each have their own `UserGameStats_<userid>_*.bin`.
   - What's unclear: Should v1 silently filter to AutoLoginUser, or fire popups for ALL users with a "[username] unlocked X" prefix?
   - Recommendation: v1 silently filters to AutoLoginUser (single-user assumption matches PROJECT.md); multi-user handling is a Phase 4 polish task.

5. **CreamAPI fixture availability**
   - What we know: No CreamAPI install on dev machine — directory does not exist.
   - What's unclear: Whether the project owner has a known game with CreamAPI installed elsewhere that can be used as a fixture.
   - Recommendation: Plan 03-02 (CreamAPI adapter) constructs synthetic test fixtures matching the Hydra-confirmed schema; integration tests use synthetic. Production validation deferred to user testing.

6. **SSE fixture availability**
   - What we know: No SSE install on dev machine.
   - What's unclear: Whether anyone in the project has access to an SSE install for empirical schema confirmation.
   - Recommendation: Plan 03-03 ships with synthetic stats.bin fixtures; production validation deferred to user reports.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (stable) | All Phase 3 work | ✓ | 1.85+ | — |
| `cargo` + `cargo tauri` | Build | ✓ | (Phase 1 verified) | — |
| Steam install (legit) for empirical Steam-legit testing | Plan 03-01 fixture verification | ✓ | `C:\Program Files (x86)\Steam` (132274694) — 166 UserGameStats files | — |
| `UserGameStats_*.bin` files (achievement state) | Plan 03-01 testing | ✓ | 166 files, mtimes 2025-04-09 to 2026-05-03 | — |
| `UserGameStatsSchema_*.bin` files (schema) | Plan 03-01 testing | ✓ | one-per-app at same path | — |
| Goldberg/gbe_fork install (used as control + dedup test) | Plan 03-04 multi-source dedup test | ✓ | `%APPDATA%\GSE Saves` — 3 saves found in Phase 1 (1455840, 1948280, 2592160) | — |
| CreamAPI install for empirical fixture | Plan 03-02 fixture verification | ✗ | — | Synthetic INI fixtures matching Hydra-confirmed schema |
| SmartSteamEmu install for empirical fixture | Plan 03-03 fixture verification | ✗ | — | Synthetic stats.bin fixtures matching Achievement-Watcher-confirmed format |
| `byteorder 1.5.x` | Steam-legit + SSE binary parsing | ✗ (not yet in Cargo.toml) | — | Hand-rolled `u32::from_le_bytes` (acceptable, more verbose) |
| `crc32fast 1.4.x` | SSE CRC32 reverse-lookup | ✗ (not yet in Cargo.toml) | — | Hand-rolled CRC32 (correctness-tedious, not recommended) |
| `gh` CLI for canonical OSS reference reads | Research only | ✓ | — | — |
| Network for `crates.io` queries | Research only | ✓ | — | — |

**Missing dependencies with no fallback:** None — all blockers have either viable fallbacks or are research-time only.

**Missing dependencies with fallback:**
- CreamAPI live install: synthetic fixtures based on canonical OSS parsers (Hydra + Achievement-Watcher). Production validation deferred to user reporting.
- SSE live install: synthetic fixtures based on Achievement-Watcher's sse.js parser. Production validation deferred to user reporting.

**Note for the planner:** absence of live CreamAPI / SSE installs on the dev machine means Plans 03-02 and 03-03 ship with fixture-only tests; first-bug-on-real-install is expected and acceptable per project's "hobby pace, polish over speed" constraint.

## Project Constraints (from CLAUDE.md)

- **Platform**: Windows-only for v1. `cfg(target_os = "windows")` is acceptable.
- **Overlay tech**: External borderless always-on-top — Phase 2 done; Phase 3 does not touch this.
- **Detection**: File watcher only; no Steam Web API in v1. **HARD constraint for Phase 3.**
- **Distribution**: Free, open-source — Phase 4 concern.
- **Goldberg / emulator stance**: PASSIVE detection only. Phase 3 reads emulator output paths if they exist; does NOT install, configure, or recommend setup. Tests synthesize fixtures rather than asking users to install emulators.
- **Customization**: Signature style locked. Phase 3 does not touch UI.
- **Pace**: Hobby project; polish over speed. Adopting fixtures-only testing for CreamAPI/SSE is acceptable for v1.
- **GSD workflow enforcement**: Edits must go through GSD commands.

**Stack-specific (Phase 1 + Phase 2 lock-ins):**
- `tauri = 2.11.1`, `notify = 8.2`, `notify-debouncer-full = 0.7`, `tokio = 1.52`, `rusqlite = 0.39 (bundled)`, `walkdir = 2.5`, `sha2 = 0.11`, `tracing = 0.1`, `dirs = 6.0`, `serde = 1.0`, `serde_json = 1.0`, `keyvalues-parser = 0.2`, `winreg = 0.56`, `windows-rs = 0.58`, `rodio = 0.22`, `sysinfo = 0.39`, `uuid = 1.23`, `async-trait = 0.1`, `anyhow = 1`, `thiserror = 2`. All locked in `src-tauri/Cargo.toml`.
- **Phase 3 NEW deps**: `byteorder = "1.5"`, `crc32fast = "1.4"`. No other version bumps.
- **WatcherCore + run_pipeline + CrossSourceDedup + SqliteStore**: locked APIs from Phase 1; Phase 3 adapters consume them unchanged.
- **SourceAdapter trait (5 methods, Send + Sync + 'static)**: locked from Plan 01-02.
- **`unlock_history.source TEXT` column + `idx_unlock_dedup` UNIQUE INDEX**: locked from Plan 01-02; absorbs new source strings.
- **Lifetime: `'static` adapters owned by `Arc`**: required by trait; new adapters mirror this.
- **Pre-flight**: Plan 03-00 (a brief spike, not a full plan) should confirm DETECT-02's path with the user OR write a CONTEXT.md note locking the `appcache/stats` decision.

## Validation Architecture

> Skipped — `workflow.nyquist_validation = false` in `.planning/config.json`. The phase still produces unit + integration tests as part of each plan's normal Self-Check, but no Nyquist gate is run.

## Security Domain

> `security_enforcement` is not configured in `.planning/config.json`. This phase introduces no authentication, authorisation, network egress (binding constraint: no Web API), or session management — all reads are local files in user-trusted paths. Threat surface is consistent with Phase 1+2 (which had explicit threat models per plan):

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes | Defensive parsing in all 3 new adapters: parse failure logs warn + skips, never panics (per Phase 1 PITFALLS pattern). Tolerate unknown VDF type tags by skipping entries. |
| V6 Cryptography | no | (CRC32 is a checksum, not crypto.) |

### Known Threat Patterns for this stack (Phase 3 additions)

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Maliciously crafted `UserGameStats_*.bin` causes parser DoS via deep recursion | DoS | Bound recursion depth in `vdf_binary.rs::read_object_body` to e.g. 16; on overflow, log + skip. (Achievement state files in the wild are depth ≤4.) |
| Maliciously crafted `stats.bin` (SSE) declares `expectedStatsCount` huge to cause OOM | DoS | Cap `count` at the smaller of `header_value` and `bytes.len() / 24`; ignore the discrepancy with a warn. |
| CreamAPI INI section name with `[<long string>]` causes large-string allocation | DoS | Cap section name length at e.g. 1024 chars; skip oversize. |
| Path traversal via attacker-controlled `%APPDATA%\CreamAPI\..\..\Windows\System32\foo.cfg` | Tampering | Adapter only READS files; never writes/executes. Watch root is statically declared (`%APPDATA%\CreamAPI`); recursive watch of that root cannot escape the directory. Per-event path is validated by `path.starts_with(watch_root)` (Phase 1 invariant). |
| File-content tampering producing fake "earned" events | Spoofing | Local-only, single-user app. Adversary with write access to user's `%APPDATA%` already controls the user. Not in scope. |
| `tracing::info!` logs include user paths + appids | Information disclosure | Local stdout only; no telemetry. Same posture as Phase 1+2 (accepted). |

Per-plan threat models (T-XX-T1 through T-XX-S1 style) belong in each Plan 03-NN, not in research. Recommended: each new adapter plan inherits the Phase 1 PITFALLS.md pattern (T-04 for Goldberg) — Plan 03-01 produces T-31, Plan 03-02 produces T-32, etc.

## Sources

### Primary (HIGH confidence — empirical or directly observed)

- **Empirical inspection of local Steam install** at `C:\Program Files (x86)\Steam\appcache\stats` — 166 `UserGameStats_132274694_*.bin` files + corresponding `UserGameStatsSchema_*.bin` files. Hex dumps via `xxd` confirmed the binary VDF format (type tags 0x00 Object, 0x01 String, 0x02 Int32, 0x08 ObjectEnd) used by both file kinds. Specific files inspected: `546560` (959 bytes), `1237970` (959 bytes), `480` (Spacewar, 94 bytes). User ID 132274694.
- **Empirical Goldberg saves** at `%APPDATA%\GSE Saves\` — 3 real saves preserved from Phase 1 (1455840, 1948280, 2592160). Format documented in `empirical-goldberg-schema-NOTES.md`.
- **Phase 1 RESEARCH.md** at `.planning/phases/01-detection-pipeline-foundation/01-RESEARCH.md` — locks the SourceAdapter trait, WatcherCore, dedup, and Goldberg adapter patterns. `Read` on 2026-05-09 of relevant sections.
- **Phase 1 plan SUMMARYs** (`01-01-` through `01-05-SUMMARY.md`) — confirm the exact integration shape, the `Vec<Arc<dyn SourceAdapter>>` Phase 3 will extend, and the `CrossSourceDedup` keying strategy.
- **Phase 1 source code** at `src-tauri/src/sources/mod.rs`, `src-tauri/src/sources/goldberg.rs`, `src-tauri/src/watcher/mod.rs`, `src-tauri/src/watcher/dedup.rs`, `src-tauri/src/paths.rs`, `src-tauri/src/lib.rs` — direct `Read` on 2026-05-09 to extract the locked APIs.
- **xan105/Achievement-Watcher source code** (LGPL-3.0) — canonical OSS reference for legit-Steam scanning (`app/parser/steam.js`), CreamAPI INI flavor (`getAchievementsFromFile()` ini-fallback chain), SmartSteamEmu `stats.bin` parser (`app/parser/sse.js`), and watch path tables (`service/watchdog/monitor.js`). Read via `gh api` on 2026-05-09.
- **hydralauncher/hydra source code** — second canonical OSS reference confirming CreamAPI path/format (`src/main/services/achievements/find-achivement-files.ts` + `parse-achievement-file.ts:processCreamAPI`) and SmartSteamEmu's alternate path (`User\Achievements.ini`). Read via `gh api` on 2026-05-09.
- **mexus/steam-vdf-parser README** — documents the type-tag table for binary VDF (0x00 Object, 0x01 String, 0x02 Int32, 0x03 Float, 0x04 Pointer, 0x05 WString, 0x06 Color, 0x07 UInt64, 0x08 ObjectEnd). Fetched via WebFetch 2026-05-09.

### Secondary (MEDIUM confidence — multiple corroborating sources)

- **CosmicHorrorDev/vdf-rs README** (`docs.rs/keyvalues_parser`) — confirmed `keyvalues-parser 0.2.3` is text-only, not applicable to binary VDF.
- **Med-Echbiy/UnlockIt** + **MrOz59/of-client-launcher** + **piradata/fork-hydra** — additional OSS references all citing the same `%APPDATA%\CreamAPI\<appid>\stats\CreamAPI.Achievements.cfg` path (high cross-source agreement).
- **`creamapi.org/`** (the project's own homepage) — confirms `%appdata%/CreamAPI/%appid%/` storage but does not document the file format (intentional, per the project's "educational" framing). WebFetch on 2026-05-09.
- **crates.io API** queries on 2026-05-09 for `byteorder` (1.5.0), `crc32fast` (1.4.2), `notify-debouncer-full` (0.7.0 still current as of 2026-05-02), `steam-vdf-parser` (0.1.1, 2026-01-18), `keyvalues-serde` (0.2.3).

### Tertiary (LOW confidence — single source or community-maintained)

- **`steamcommunity.com` "Steam Achievement Statistics" guide** — describes the `UserGameStatsSchema` file presence and high-level structure but not the byte-level format. Confirms our hand-rolling decision (no community-published reverse engineering exists).
- **Achievement Watcher Wiki Compatibility page** — high-level paths only; not byte-level.
- **`gibbed/SteamAchievementManager` issue tracker** — confirms `IncrementOnly` flag exists in schema files (relevant only as defensive parsing — Phase 3 reads stats; doesn't write them, so the flag doesn't matter).

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — all crates verified against crates.io API on 2026-05-09; Phase 1 stack lock-ins directly inspected.
- Architecture: HIGH — re-read of locked Phase 1 source confirms additive integration is straightforward.
- Steam-legit binary VDF schema: HIGH for state file (empirically inspected); MEDIUM-HIGH for schema file (1 schema file deeply inspected, 165 not deep-inspected).
- CreamAPI INI schema: HIGH — confirmed by 4+ canonical OSS parsers in agreement.
- SmartSteamEmu schema: MEDIUM — single canonical source (Achievement-Watcher); secondary references disagree on path.
- Cross-source dedup correctness for N sources: HIGH — Phase 1 source code keys on `(app_id, ach_api_name)` which is source-agnostic by construction.
- Path discovery extension: HIGH — additive, no breaking concerns.
- Pitfalls: HIGH — all 9 pitfalls drawn from canonical sources or empirical inspection.
- Hand-roll justifications: HIGH — verified absence of mature crates for these specific formats.

**Research date:** 2026-05-09
**Valid until:** 2026-06-08 (30 days — stable Rust ecosystem; type tags + INI/binary file formats are decade-stable). Re-check `notify-debouncer-full` and `byteorder` versions if planning starts after this date.
