---
phase: 03-remaining-source-adapters
plan: "00"
subsystem: infra
tags: [rust, tauri, byteorder, crc32fast, source-adapters, stub-first, vdf-binary]

requires:
  - phase: 01-detection-pipeline-foundation
    provides: SourceAdapter trait, SourceKind::Goldberg, DiscoveredPaths struct, paths::discover()
  - phase: 02-premium-ui-popup-companion-game-session
    provides: Schema cache, lib.rs::run() pipeline call site
provides:
  - Empirical confirmation of <SteamPath>\appcache\stats path + binary VDF \0cache\0 header
  - REQUIREMENTS.md DETECT-02 path correction (away from misleading userdata/<steamid>/<appid>/remote/)
  - Cargo deps: byteorder 1.5 + crc32fast 1.4
  - SourceKind enum extended with SteamLegit, CreamApi, SmartSteamEmu (stable lowercase as_str)
  - Stub modules sources/{steam_legit,cream_api,sse,vdf_binary}.rs with discover_paths helpers
  - DiscoveredPaths struct extended with 4 new fields (steam_legit_appcache_stats, steam_legit_user_ids, cream_api_appid_dirs, sse_appid_dirs) wired into discover() + log_discovery()
affects: [03-01-steam-legit-adapter, 03-02-cream-api-adapter, 03-03-sse-adapter, 03-04-pipeline-integration]

tech-stack:
  added: [byteorder 1.5, crc32fast 1.4]
  patterns:
    - "Stub-first module declaration (Phase 2 Plan 02-01 pattern) — sources/mod.rs declares all 4 new modules upfront so Wave 2 plans modify only file contents, never the module list"
    - "Phase-local empirical NOTES.md (Phase 1 Plan 01-01 pattern) — capture dev-machine ground truth so adapter implementers don't redo path research"
    - "DiscoveredPaths uses ..Default::default() in test struct literals — adding new fields no longer breaks tests"

key-files:
  created:
    - .planning/phases/03-remaining-source-adapters/empirical-vdf-NOTES.md
    - src-tauri/src/sources/steam_legit.rs
    - src-tauri/src/sources/cream_api.rs
    - src-tauri/src/sources/sse.rs
    - src-tauri/src/sources/vdf_binary.rs
  modified:
    - .planning/REQUIREMENTS.md
    - src-tauri/Cargo.toml
    - src-tauri/src/sources/mod.rs
    - src-tauri/src/paths.rs
    - src-tauri/tests/integration_phase1.rs

key-decisions:
  - "Plan 03-00: REQUIREMENTS.md DETECT-02 corrected — achievement state lives at appcache/stats/UserGameStats_<userid>_<appid>.bin, NOT userdata/<steamid>/<appid>/remote/ (which is Steam Cloud save data)"
  - "Plan 03-00: SourceKind::SmartSteamEmu.as_str() = 'smartsteamemu' (single token, no separator) — stable for SQLite TEXT column"
  - "Plan 03-00: Stub-first declaration of vdf_binary.rs at module scope (not nested under steam_legit) — keeps the binary KV reader reusable for future SSE schema needs"
  - "Plan 03-00: Existing DiscoveredPaths struct literals in tests updated to use ..Default::default() — minimum-surface change as new fields are appended"

patterns-established:
  - "Stub-first scaffolding: when a phase will add N adapters in Wave 2, the pre-flight plan declares all N modules with no-op SourceAdapter impls so each Wave 2 plan modifies only its single owned source file"
  - "Empirical NOTES.md companion: phases that resolve a research-time path/schema unknown produce a phase-local NOTES.md with reproducible PowerShell stdout + hex dumps"

requirements-completed: []

duration: 12min
completed: 2026-05-09
---

# Phase 03 Plan 00: Pre-flight Spike Summary

**Resolved DETECT-02 path bug, captured Steam binary VDF empirical evidence, and laid stub-first scaffolding (Cargo deps, SourceKind variants, 4 stub source modules, DiscoveredPaths struct extension) so plans 03-01/02/03 run cleanly in Wave 2 without touching shared files.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-05-09T08:40:00Z
- **Completed:** 2026-05-09T08:52:00Z
- **Tasks:** 2
- **Files modified:** 5 (REQUIREMENTS.md, Cargo.toml, mod.rs, paths.rs, integration_phase1.rs)
- **Files created:** 5 (empirical-vdf-NOTES.md, steam_legit.rs, cream_api.rs, sse.rs, vdf_binary.rs)

## Accomplishments

- **DETECT-02 path bug fixed.** REQUIREMENTS.md no longer cites the misleading `userdata/<steamid>/<appid>/remote/` Steam-Cloud-save path; it now points to the empirically-verified `<SteamPath>\appcache\stats\UserGameStats_<userid>_<appid>.bin` location with cross-reference to the dev-machine ground-truth NOTES.
- **Empirical evidence captured.** PowerShell scan of dev machine confirmed 3 real `UserGameStats_132274694_<appid>.bin` files at 78–191 bytes alongside their `UserGameStatsSchema_<appid>.bin` counterparts at 18 KB–41 KB. 32-byte hex dump (`00 63 61 63 68 65 00 02 63 72 63 00 CD 2C 30 DB ...`) confirms binary VDF `\0cache\0` header + Int32 type-tag (0x02) crc field exactly as RESEARCH.md predicted.
- **Cargo deps added.** `byteorder = "1.5"` (binary VDF reader) + `crc32fast = "1.4"` (SmartSteamEmu API-name CRC reverse-lookup), pinned to current stable.
- **SourceKind enum extended.** Three new variants `SteamLegit`, `CreamApi`, `SmartSteamEmu` with stable lowercase `as_str()` values (`"steam_legit"`, `"cream_api"`, `"smartsteamemu"`) and updated test asserting all four.
- **4 stub source files created.** Each has a `discover_paths()` helper and an adapter struct that implements `SourceAdapter` with no-op `seed_baseline`/`on_file_changed`. Bodies populated by plans 03-01/02/03; the trait surface is locked. Smoke tests assert default-empty discovery and adapter `kind()`/`name()` values.
- **DiscoveredPaths struct extended.** 4 new fields (`steam_legit_appcache_stats: Option<PathBuf>`, `steam_legit_user_ids: Vec<u64>`, `cream_api_appid_dirs: Vec<PathBuf>`, `sse_appid_dirs: Vec<PathBuf>`) wired into both `discover()` (calls sub-module `discover_paths()`) and `log_discovery()` (4 new info-level log statements).
- **All tests still pass.** `cargo test --lib` reports `106 passed; 0 failed; 0 ignored` after this plan (vs the Phase 2 baseline).

## Task Commits

Each task was committed atomically:

1. **Task 1: Empirical re-validation + REQUIREMENTS.md fix + NOTES.md creation** — `b8d2519` (docs)
2. **Task 2: Cargo deps + SourceKind enum extension + stub modules + DiscoveredPaths extension** — `eb99a8b` (feat)

## REQUIREMENTS.md Diff (DETECT-02)

**Before:**
```
- [ ] **DETECT-02**: Real-time watcher detects unlocks from legitimate Steam (binary VDF parser of `userdata/<steamid>/<appid>/remote/`, mtime trigger via `appcache/stats`)
```

**After:**
```
- [ ] **DETECT-02**: Real-time watcher detects unlocks from legitimate Steam (binary VDF parser of `<SteamPath>\appcache\stats\UserGameStats_<userid>_<appid>.bin` for achievement state, with `<SteamPath>\appcache\stats\UserGameStatsSchema_<appid>.bin` for stat-slot to API-name mapping). NOTE: Originally cited `userdata/<steamid>/<appid>/remote/` — that is Steam Cloud save storage, not achievement state. Corrected 2026-05-09 per `.planning/phases/03-remaining-source-adapters/empirical-vdf-NOTES.md`.
```

## Files Created/Modified

- `.planning/REQUIREMENTS.md` — DETECT-02 line corrected; trailing NOTE points readers at empirical-vdf-NOTES.md.
- `.planning/phases/03-remaining-source-adapters/empirical-vdf-NOTES.md` — verbatim PowerShell stdout (3 state files + 3 schema files), 32-byte hex dump, type-tag inventory, and decision-for-Plan-03-01 section.
- `src-tauri/Cargo.toml` — `byteorder = "1.5"` + `crc32fast = "1.4"` added between `reqwest` and the windows-target dependencies block.
- `src-tauri/src/sources/mod.rs` — 4 `pub mod` declarations; `SourceKind` enum extended with 3 variants + stable lowercase `as_str()` arms; existing `source_kind_as_str_is_stable_lowercase` test extended to assert all 4 variants.
- `src-tauri/src/sources/steam_legit.rs` — stub adapter, `SteamLegitPaths` struct (appcache_stats + user_ids), `discover_paths(steam_install)` returning default, 2 smoke tests.
- `src-tauri/src/sources/cream_api.rs` — stub adapter, `CreamApiPaths` struct (appid_dirs), `discover_paths()` returning default, 2 smoke tests; comment block notes Plan 03-02 will honor `HALLMARK_CREAMAPI_ROOT_OVERRIDE` env var.
- `src-tauri/src/sources/sse.rs` — stub adapter, `SsePaths` struct (appid_dirs), `discover_paths()` returning default, 2 smoke tests; comment block notes Plan 03-03 will honor `HALLMARK_SSE_ROOT_OVERRIDE` env var.
- `src-tauri/src/sources/vdf_binary.rs` — `Value` enum (Object/String/Int32/Float/UInt64), `Vdf` struct, `parse_binary_vdf(&[u8]) -> anyhow::Result<Vdf>` returning Err in stub; 1 smoke test.
- `src-tauri/src/paths.rs` — `DiscoveredPaths` extended with 4 new fields; `discover()` calls 3 sub-module `discover_paths()` helpers; `log_discovery()` adds 4 new info-level log blocks; 3 in-file test struct-literal sites updated to use `..Default::default()`.
- `src-tauri/tests/integration_phase1.rs` — 2 integration-test struct-literal sites updated to use `..Default::default()`.

## Decisions Made

- **Use `..Default::default()` everywhere** — instead of explicitly setting all 4 new fields to their defaults at every test struct-literal, used Rust's struct-update syntax. Future field additions in Plan 03-04 won't require touching tests.
- **`vdf_binary.rs` placed at sources/ scope, not nested** — keeps the parser reusable should SmartSteamEmu's binary stats need any common type-tag handling later.
- **`SourceKind::SmartSteamEmu.as_str()` = `"smartsteamemu"`** — single lowercase token (no separator). Matches the SourceAdapter `name()` value, easy to query in SQLite.
- **Filename-pattern guard documented in NOTES, not coded yet** — the regex `^UserGameStats_(\d+)_(\d+)\.bin$` is documented as Plan 03-01's responsibility; the stub adapter's `on_file_changed` is a no-op.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Updated existing DiscoveredPaths struct literals in tests to use `..Default::default()`**
- **Found during:** Task 2 (after extending the struct, `cargo check --all-targets` failed with E0063 "missing fields" at 5 test struct-literal sites — 3 in `paths.rs::tests_*`, 2 in `tests/integration_phase1.rs`).
- **Issue:** Adding 4 new fields to `DiscoveredPaths` broke every existing test that constructed a struct literal. The plan listed `paths.rs` in the `<files>` block but did not enumerate the 5 sites needing updates.
- **Fix:** Appended `..Default::default()` to each struct literal — minimum-surface change. The struct already derives `Default`.
- **Files modified:** `src-tauri/src/paths.rs` (3 sites), `src-tauri/tests/integration_phase1.rs` (2 sites).
- **Verification:** `cargo check --all-targets` exits 0; `cargo test --lib` reports 106 passed.
- **Committed in:** `eb99a8b` (Task 2 commit).

---

**Total deviations:** 1 auto-fixed (Rule 3 — Blocking).
**Impact on plan:** Necessary to compile after struct extension. No scope creep — all changes are mechanical and minimal.

## Issues Encountered

- PowerShell-via-Bash escaping clobbered `$bytes`, `$first`, etc. variables when running the hex-dump command inline. Resolved by writing a temporary `hex_dump.ps1` script, running it via `powershell -File`, then deleting the script. The captured output (`00 63 61 63 68 65 00 ...`) is verbatim in `empirical-vdf-NOTES.md`.

## Next Phase Readiness

- **Plan 03-01 (SteamLegitAdapter):** Has empirical-vdf-NOTES.md (path + schema-file companion + hex header + type tags) + a stub `steam_legit.rs` it only needs to populate the body of. No Cargo edits, no struct changes, no enum changes required.
- **Plan 03-02 (CreamApiAdapter):** Has a stub `cream_api.rs` to populate; the env-var override pattern is documented in the stub comment block. No Cargo / struct / enum edits needed.
- **Plan 03-03 (SseAdapter):** Has a stub `sse.rs` to populate plus the `crc32fast` crate already on Cargo.toml. The Hydra `User\Achievements.ini` variant disposition (warn-and-skip) is documented in the stub comment block. No Cargo / struct / enum edits needed.
- **Plan 03-04 (Pipeline integration):** Adapter Vec wiring in `lib.rs::run()` will only need to append 3 `.into()` constructions. The 3-source cross-source dedup integration test will pass — `CrossSourceDedup` was already designed in Plan 01-05 to key on `(app_id, ach_api_name)` and generalize to N adapters.

## Self-Check: PASSED

Verified each created file exists:
- `.planning/REQUIREMENTS.md` — FOUND (DETECT-02 corrected)
- `.planning/phases/03-remaining-source-adapters/empirical-vdf-NOTES.md` — FOUND (61 lines)
- `src-tauri/Cargo.toml` — FOUND (byteorder 1.5 + crc32fast 1.4)
- `src-tauri/src/sources/mod.rs` — FOUND (4 mod decls + 3 enum variants)
- `src-tauri/src/sources/steam_legit.rs` — FOUND
- `src-tauri/src/sources/cream_api.rs` — FOUND
- `src-tauri/src/sources/sse.rs` — FOUND
- `src-tauri/src/sources/vdf_binary.rs` — FOUND
- `src-tauri/src/paths.rs` — FOUND (4 new fields + discover() integration + log_discovery() additions)

Verified each commit exists:
- `b8d2519` — FOUND (Task 1: REQUIREMENTS.md fix + NOTES.md)
- `eb99a8b` — FOUND (Task 2: Cargo deps + stubs + DiscoveredPaths extension)

Verified `cargo check --all-targets` exits 0 and `cargo test --lib` reports `106 passed; 0 failed`.

---
*Phase: 03-remaining-source-adapters*
*Completed: 2026-05-09*
