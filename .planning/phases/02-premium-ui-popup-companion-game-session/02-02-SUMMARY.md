---
phase: 02-premium-ui-popup-companion-game-session
plan: "02"
subsystem: schema
tags: [sqlite, schema-cache, steam-api, goldberg, reqwest, async, rust]

# Dependency graph
requires:
  - phase: 02-premium-ui-popup-companion-game-session
    provides: "schema/ stub files from Plan 01, store/queries.rs Phase 1 helpers, schema_cache + companion_prefs SQLite tables from migration 002"

provides:
  - "SchemaCache orchestrator with D-24 two-leg resolution chain (Goldberg metadata → public Steam API rarity)"
  - "AchievementSchema public type (serde Serialize) for Plans 05/06 IPC"
  - "classify_tier(global_pct) — rare (<10%) vs standard with D-07 graceful degrade"
  - "cache.rs typed SQLite helpers: upsert_schema, get_schema_row, get_schema_for_app, schema_count_for_app"
  - "queries.rs Phase 2 helpers: mark_completion_fired, is_completion_fired, CompanionPrefs struct + CRUD, count_earned_for_app_session"
  - "steam_api.rs: async fetch_global_pcts with public no-key endpoint only"
  - "appcache.rs: find_local_icon for Steam librarycache game-header icons"
  - "goldberg_meta.rs: tolerant parse_goldberg_metadata for both array and object shapes + field-name variants"

affects:
  - 02-03 (game_detect triggers resolve() on game-start; supplies goldberg_json_paths)
  - 02-05 (popup_queue calls lookup() at fire-time; count_earned_for_app_session for 100% trigger)
  - 02-06 (companion calls list_for_app(); receives schema-resolved events)

# Tech tracking
tech-stack:
  added:
    - "reqwest::Client with 8s timeout + Hallmark User-Agent for Steam API calls"
  patterns:
    - "Read-merge-upsert pattern: each resolve() leg reads existing row, merges with new fields, upserts — preserves fields from other legs"
    - "Non-blocking async resolve: tokio::spawn at game-start; popups never wait; schema-resolved event upgrades companion in-place"
    - "Tolerant field-variant parsing: serde_json::Value with cascading .or_else() for Goldberg field-name inconsistencies"
    - "Per-leg independent failure: warn! + continue so partial cache is always better than no cache"

key-files:
  created: []
  modified:
    - src-tauri/src/schema/cache.rs (replaced stub — SchemaCacheRow + 4 typed helpers + 4 tests)
    - src-tauri/src/store/queries.rs (appended 5 Phase 2 helpers + CompanionPrefs struct + 3 tests)
    - src-tauri/src/schema/steam_api.rs (replaced stub — fetch_global_pcts, no-key enforcement, 1 test)
    - src-tauri/src/schema/appcache.rs (replaced stub — find_local_icon, 4 tests)
    - src-tauri/src/schema/goldberg_meta.rs (replaced stub — parse_goldberg_metadata, GoldbergAchievementMeta, 6 tests)
    - src-tauri/src/schema/mod.rs (replaced stub — SchemaCache, AchievementSchema, classify_tier, 7 tests)

key-decisions:
  - "Read-merge-upsert (not COALESCE in SQL): each resolve leg reads existing row, merges at Rust level, upserts — verbose SQL kept simple; callers always have full row at write time"
  - "steam_api.rs calls ONLY GetGlobalAchievementPercentagesForApp/v0002/ — no GetSchemaForGame, no API key; T-02-11 mitigation enforced via acceptance criterion grep"
  - "appcache.rs stores icon_path as filesystem path (not BLOB) per D-11 decision from Plan 01 — librarycache covers game-header art only; achievement icons come from Goldberg icon_url field"
  - "resolve() emits two schema-resolved events: one after Goldberg metadata leg, one after rarity leg — allows companion to show name/description immediately without waiting for network"
  - "read_with_retry retries on raw_os_error() 32 AND 33 (sharing violation variants) to match goldberg.rs pattern"

patterns-established:
  - "Schema resolution is non-blocking: lookup() is synchronous + cheap (one SQLite read); resolve() is async and spawnable"
  - "classify_tier is a pure function returning &'static str — plugs directly into PopupPayload.tier without allocation"
  - "Goldberg metadata tolerant parsing: cascading .or_else() chains try all known field-name variants before returning None"

requirements-completed: [GAME-02, GAME-03, POPUP-07]

# Metrics
duration: ~25min
completed: 2026-05-08
---

# Phase 2 Plan 02: Schema Resolution Chain Summary

**Non-blocking SchemaCache with D-24 two-leg resolution (Goldberg metadata + public Steam API rarity), typed SQLite helpers for schema_cache/companion_prefs, and tolerant Goldberg JSON parser — 22 schema tests + 3 new query tests all pass**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-05-08
- **Completed:** 2026-05-08
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- Built SchemaCache orchestrator implementing D-24 lookup chain: Goldberg metadata leg then public Steam Web API rarity leg, each independently fallible with warn+continue; emits `schema-resolved` event after each leg for companion in-place upgrade
- Replaced all 5 schema stub files with real implementations: cache.rs (4 typed SQLite helpers), steam_api.rs (no-key public endpoint), appcache.rs (librarycache icon discovery), goldberg_meta.rs (tolerant variant parser), mod.rs (orchestrator)
- Appended 5 Phase 2 helpers to store/queries.rs (mark_completion_fired, is_completion_fired, CompanionPrefs CRUD, count_earned_for_app_session) without touching the 4 Phase 1 helpers
- 22 schema tests + 3 new query tests all pass; `cargo build -p hallmark` exits 0

## Task Commits

Each task was committed atomically:

1. **Task 1: SQLite query helpers — schema_cache + completion + companion_prefs** - `bda3d63` (feat)
2. **Task 2: Steam API client + appcache reader + Goldberg meta parser** - `502ba05` (feat)
3. **Task 3: SchemaCache orchestrator** - `90d048c` (feat)

## Files Created/Modified

- `src-tauri/src/schema/cache.rs` — SchemaCacheRow struct + upsert_schema, get_schema_row, get_schema_for_app, schema_count_for_app; 4 unit tests
- `src-tauri/src/store/queries.rs` — appended mark_completion_fired, is_completion_fired, CompanionPrefs struct, set_companion_prefs, get_companion_prefs, count_earned_for_app_session; 3 unit tests
- `src-tauri/src/schema/steam_api.rs` — async fetch_global_pcts, no API key; url_contains_no_api_key_marker test guards the constraint
- `src-tauri/src/schema/appcache.rs` — find_local_icon with preferred-filename + prefix-fallback logic; 4 unit tests
- `src-tauri/src/schema/goldberg_meta.rs` — GoldbergAchievementMeta + parse_goldberg_metadata; tolerates array/object shapes and displayName/display_name/desc/iconUrl variants; 6 unit tests
- `src-tauri/src/schema/mod.rs` — SchemaCache (new, lookup, list_for_app, resolve), AchievementSchema (Serialize), classify_tier; 7 unit tests including tokio::test for async paths

## Decisions Made

- Read-merge-upsert pattern at Rust level rather than SQL COALESCE per-column: simpler to read, callers always have full row at write time, correct merge semantics across legs
- Two schema-resolved events (one after metadata leg, one after rarity leg) so companion can render names immediately without waiting for the network call
- raw_os_error() 32 AND 33 checked in read_with_retry to match goldberg.rs pattern — both variants of Windows sharing violation observed in practice

## Deviations from Plan

None - plan executed exactly as written. All three tasks implemented per spec; 22 schema tests (4 cache + 4 appcache + 6 goldberg_meta + 1 steam_api + 7 mod) and 3 new queries tests pass.

## Known Stubs

None — all schema stub files replaced with real implementations. The `resolve()` method's `app` parameter (tauri::AppHandle) is wired but not tested in unit tests (requires live Tauri runtime); Plan 03 integration will exercise the full game-start trigger path.

## Threat Surface

| Flag | File | Description |
|------|------|-------------|
| threat_flag: outbound-network | src-tauri/src/schema/steam_api.rs | Outbound HTTPS to api.steampowered.com. No API key; rustls-tls validates cert. T-02-08 (DNS spoofing) accepted; T-02-13 (slow-loris) mitigated by 8s reqwest timeout |
| threat_flag: user-controlled-input | src-tauri/src/schema/goldberg_meta.rs | Goldberg achievements.json is user-writable. T-02-09 (malformed JSON) mitigated — serde_json returns Result; extract() returns Option per achievement; malformed_json_returns_err test verifies. T-02-10 (100MB payload) accepted for Phase 2. |

## Issues Encountered

None — `cargo check`, all tests, and `cargo build -p hallmark` all passed on first attempt.

## User Setup Required

None — no external service configuration required. Schema resolution is triggered internally on game-start; Steam API call requires internet but fails gracefully (warn + continue).

## Next Phase Readiness

- `pub mod schema` (declared in lib.rs by Plan 01) now exposes `SchemaCache`, `AchievementSchema`, and `classify_tier` for Plans 05/06
- Plan 03 (game_detect) can call `schema_cache.resolve(app, app_id, goldberg_paths).await` on game-start
- Plan 05 (popup_queue) can call `schema_cache.lookup(app_id, ach_api_name)` at fire-time and `count_earned_for_app_session` for 100% trigger
- Plan 06 (companion) can call `schema_cache.list_for_app(app_id)` and listen for `schema-resolved` events
- Concern: resolve() calls `std::fs::read_to_string` (blocking I/O) inside an async function — acceptable for Phase 2 (single file per path, rare call), but Plan 03 should evaluate `tokio::fs::read_to_string` if blocking is observed in practice

---
*Phase: 02-premium-ui-popup-companion-game-session*
*Completed: 2026-05-08*
