---
phase: 03-remaining-source-adapters
verified: 2026-05-09T13:05:00Z
status: human_needed
score: 3/3 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: none
  previous_score: ""
  gaps_closed: []
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "ROADMAP SC#1 — full-pipeline latency on a real Steam install"
    expected: "Real achievement unlock from a legitimate Steam game (with Steam client running) fires Hallmark popup within one second, no manual path config"
    why_human: "Automated tests use synthetic UserGameStats_*.bin fixtures and bypass notify-debouncer-full (test sc1_steam_legit_emits_event_synchronously was renamed and DROPPED the <1s latency assertion per WR-05). The full-pipeline 1s SLA across debounce + dispatch + dedup + popup-render is not asserted by any automated test against a real Steam library. Requires a machine with a real Steam install, an active game, and an actual achievement unlock to confirm."
  - test: "ROADMAP SC#2 — real-world auto-discovery on a user machine"
    expected: "On a machine that has CreamAPI and/or SmartSteamEmu installed, paths::discover() returns the correct appid_dirs without HALLMARK_*_OVERRIDE env vars set"
    why_human: "Automated SC2 test only verifies discovery against fixture trees via env-var override (HALLMARK_CREAMAPI_ROOT_OVERRIDE / HALLMARK_SSE_ROOT_OVERRIDE). The real production lookup at %APPDATA%\\CreamAPI and %APPDATA%\\SmartSteamEmu is not exercised. A user with real installations of either emulator should confirm discover_paths() finds them on first launch."
  - test: "ROADMAP SC#3 — concurrent multi-emulator real-world scenario"
    expected: "If a real game has both legit Steam achievements + Goldberg/CreamAPI emulator running alongside, exactly one popup fires per logical unlock"
    why_human: "Automated sc3_three_source_simultaneous_unlock_collapses_to_one_popup uses MockAdapters with a synthetic shared payload, and sc3_supplement_real_three_source_endtoend uses real adapters with synthetic fixtures. Neither exercises the actual case of legit Steam + emulator running simultaneously against a real game. Optional spot-check by user with such a setup."
  - test: "Popup quality parity with Goldberg unlocks"
    expected: "Popup shown for SteamLegit/CreamApi/SmartSteamEmu unlocks is visually + audibly identical to popup shown for Goldberg unlocks (same icon, animation, sound, tier styling)"
    why_human: "Phase 3 verifies events flow through the same RawUnlockEvent pipeline (architectural parity), but visual + audible parity is a UX claim that requires human confirmation across all 4 source kinds. Phase 2 popup_queue accepts events regardless of source — automated tests cannot validate the rendered output."
deferred: []
---

# Phase 3: Remaining Source Adapters — Verification Report

**Phase Goal:** Achievement unlocks from legitimate Steam installations (binary VDF), CreamAPI, and SmartSteamEmu all flow through the same pipeline and fire the same premium popup as Goldberg, with no duplicate popups when multiple adapters observe the same logical unlock.

**Verified:** 2026-05-09T13:05:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Verdict

The architectural goal of Phase 3 — three new adapters wired through the same pipeline as Goldberg, with cross-source dedup proven to generalize from 2 to N adapters — is fully achieved in code. All three success criteria pass automated verification at the architectural / mechanical level: 4 adapters constructed in `lib.rs::run()`, integration tests prove DETECT-02/03/04 emit events with correct SourceKinds, and the headline 3-source dedup test asserts exactly ONE event + ONE SQLite row from three near-simultaneous emits.

However, all three ROADMAP success criteria reference real-world conditions ("legitimate Steam game with Steam client running," "auto-detected by path discovery," "running alongside") that cannot be exercised by automated tests using synthetic fixtures. Critically, the SC1 latency assertion was intentionally dropped during code review (WR-05) because the test bypassed the debouncer; the production 1-second SLA is therefore not verified by any automated check. Status is `human_needed` to surface these real-world UAT gaps.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Steam-legit unlock fires the same popup pipeline as Goldberg, no manual path config | VERIFIED (architectural) | `src-tauri/src/lib.rs:201-205` constructs SteamLegitAdapter from `discovery.steam_legit_appcache_stats` (auto-discovered via HKCU registry per `steam_legit.rs:69-85`); `integration_phase3.rs::sc1_steam_legit_emits_event_synchronously` proves emit produces `RawUnlockEvent { source: SourceKind::SteamLegit }` end-to-end; SC4 confirms 4-adapter Vec at `lib.rs:214-219`. **GAP:** the 1-second latency claim is no longer asserted by any test (WR-05 dropped it); requires human UAT. |
| 2 | CreamAPI and SmartSteamEmu installs auto-detected and fire the same premium popup | VERIFIED (against fixtures) | `cream_api.rs:53-89` enumerates `%APPDATA%\CreamAPI\<numeric_appid>\stats\CreamAPI.Achievements.cfg`; `sse.rs:67-103` enumerates `%APPDATA%\SmartSteamEmu\<numeric_appid>\stats.bin`; `integration_phase3.rs::sc2_cream_api_and_sse_paths_auto_discovered` (lines 295-437) drives both adapters end-to-end against fixture trees with HALLMARK_*_OVERRIDE — both adapters emit RawUnlockEvent for app_id 4242 with correct SourceKinds. **GAP:** real-world `%APPDATA%` discovery on a user machine with CreamAPI/SSE installed is not automated; requires UAT. |
| 3 | Three concurrent same-logical-unlock observations collapse to exactly one popup | VERIFIED | `integration_phase3.rs::sc3_three_source_simultaneous_unlock_collapses_to_one_popup` (lines 444-518) — three MockAdapters with kinds SteamLegit/CreamApi/SmartSteamEmu fire the same `(777, "ACH_TRIPLE_OBSERVED")` payload; pipeline emits exactly 1 sink event AND inserts exactly 1 unlock_history row. `sc3_supplement_real_three_source_endtoend` (lines 537-787) repeats the assertion with REAL SteamLegit + CreamApi + Sse adapters parsing synthetic fixtures (per WR-04 fix counts ALL events for app_id, catching dedup leaks). |

**Score:** 3/3 truths verified (with real-world UAT gaps surfaced as human_needed).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/sources/vdf_binary.rs` | Hand-rolled binary VDF reader, all 9 type tags 0x00-0x08, recursion-bounded | VERIFIED | 281 lines; `MAX_RECURSION_DEPTH = 16`; type tags 0x00..0x08 handled; WR-02 fix asserts non-root EOF returns Err. |
| `src-tauri/src/sources/steam_legit.rs` | SteamLegitAdapter, AppSchema mtime cache, registry discover_paths | VERIFIED | 760 lines; `impl SourceAdapter for SteamLegitAdapter` at line 346; `extract_state_mapping` + `extract_schema_mapping` present; HKCU `Software\Valve\Steam\Users` enumerated at line 64; CR-01/CR-02 fixes applied. |
| `src-tauri/src/sources/cream_api.rs` | CreamApiAdapter, INI parser, %APPDATA%\CreamAPI discovery | VERIFIED | 487 lines; `parse_creamapi_state` at line 92 with BOM strip + comment skip + 1/true equivalence; `extract_app_id` at line 143; `STATE_FILENAME = "CreamAPI.Achievements.cfg"` at line 39; HALLMARK_CREAMAPI_ROOT_OVERRIDE supported at line 53; case-insensitive filename guard at line 227 (WR-06 fix); WR-08 empty-section rejection at line 382 test. |
| `src-tauri/src/sources/sse.rs` | SseAdapter, 24-byte record parser, lazy CRC32 reverse-lookup | VERIFIED | 610 lines; `parse_sse_stats` at line 115 (header + N×24-byte records, value>1 skip, size cap); `build_crc_reverse_map` at line 159 with 8-char zero-padded hex; HALLMARK_SSE_ROOT_OVERRIDE at line 68; Goldberg companion harvest in `load_goldberg_companion_keys`. |
| `src-tauri/src/paths.rs` | DiscoveredPaths extended; discover() + log_discovery() integrated | VERIFIED | Lines 59-65 add 4 new fields (steam_legit_appcache_stats, steam_legit_user_ids, cream_api_appid_dirs, sse_appid_dirs); `discover()` calls 3 sub-module discover_paths() at lines 81-83; `log_discovery()` logs 4 new categories at lines 142-152. |
| `src-tauri/src/lib.rs` | run() constructs all 4 adapters; tracing logs adapter_count=4 | VERIFIED | Lines 195-220: 4 Arc<dyn SourceAdapter> constructed and pushed to Vec; `tracing::info!(adapter_count = adapters.len(), "Phase 3: 4-adapter pipeline configured")` at line 220. |
| `src-tauri/tests/integration_phase3.rs` | 5 #[tokio::test]s for SC1, SC2, SC3, SC3-supplement, SC4 | VERIFIED | 848 lines; all 5 tests present and passing; MockAdapter implements SourceAdapter at line 111; EnvGuard RAII at lines 261-285; env_override_lock serializes SC2 + SC3-supplement (WR-03 fix). |
| `src-tauri/tests/fixtures/steam_legit/` | At least 1 real UserGameStats_*.bin + companion schema | VERIFIED | 4 files: `UserGameStats_132274694_{480,2670630}.bin` + `UserGameStatsSchema_{480,2670630}.bin`. |
| `.planning/phases/03-remaining-source-adapters/empirical-vdf-NOTES.md` | Empirical PowerShell stdout + hex dump | VERIFIED | File present (3,763 bytes). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `lib.rs::run::setup` | `SteamLegitAdapter::new` | `Arc::new(...)` push to adapters Vec | WIRED | lib.rs:201-205 |
| `lib.rs::run::setup` | `CreamApiAdapter::new` | `Arc::new(...)` push to adapters Vec | WIRED | lib.rs:206-209 |
| `lib.rs::run::setup` | `SseAdapter::new` | `Arc::new(...)` push to adapters Vec | WIRED | lib.rs:210-213 |
| `lib.rs::run::setup` | `watcher::run_watcher(adapters, raw_tx)` | `tauri::async_runtime::spawn` | WIRED | lib.rs:256 |
| `paths.rs::discover` | `sources::steam_legit::discover_paths` | Direct fn call | WIRED | paths.rs:81 |
| `paths.rs::discover` | `sources::cream_api::discover_paths` | Direct fn call | WIRED | paths.rs:82 |
| `paths.rs::discover` | `sources::sse::discover_paths` | Direct fn call | WIRED | paths.rs:83 |
| `steam_legit.rs::on_file_changed` | `vdf_binary::parse_binary_vdf` | Direct fn call | WIRED | Confirmed via grep — 4 call sites |
| `cream_api.rs::on_file_changed` | `parse_creamapi_state` | Direct fn call | WIRED | cream_api.rs:265 |
| `sse.rs::on_file_changed` | `parse_sse_stats` | Direct fn call | WIRED | sse.rs:353 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| SteamLegitAdapter | RawUnlockEvent | `vdf_binary::parse_binary_vdf` reading actual UserGameStats binary, diff against `Arc<RwLock<HashMap<(u64,String),bool>>> baseline` | YES — fixture test parses real 85-byte UserGameStats_132274694_480.bin without error and emits events on diff | FLOWING |
| CreamApiAdapter | RawUnlockEvent | `parse_creamapi_state` reading INI, diff against baseline | YES — multiple unit + integration tests verify INI → HashMap → event emission | FLOWING |
| SseAdapter | RawUnlockEvent | `parse_sse_stats` reading binary records, lazy CRC reverse-lookup, diff against baseline | YES — synthetic round-trip + integration test prove CRC → api_name → event emission | FLOWING |
| `lib.rs::adapters` Vec | 4-element Vec<Arc<dyn SourceAdapter>> | Constructed in setup() from real `paths::discover()` result | YES — sc4 test re-constructs the same Vec, asserts len == 4 + distinct names + distinct kinds | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Phase 3 integration tests pass | `cargo test --manifest-path src-tauri/Cargo.toml --test integration_phase3` | `5 passed; 0 failed; 0 ignored; finished in 5.09s` | PASS |
| Lib tests pass (no regressions) | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | `132 passed; 0 failed; 0 ignored; finished in 1.73s` (matches REVIEW.md baseline 131 + 1 WR-08 test) | PASS |
| Phase 1 integration tests still pass | `cargo test --manifest-path src-tauri/Cargo.toml --test integration_phase1` | `5 passed; 0 failed; 0 ignored; finished in 3.01s` | PASS |
| 4-adapter wiring in production lib.rs | grep `let adapters = vec!\[` + 4 `_adapter,` lines | All 4 adapter Arc constructions present at lib.rs:201-219 | PASS |
| Real fixture parses without error | sc1 test loads + parses real UserGameStats_132274694_*.bin | sc1 passes (parse → diff → emit succeeds) | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| DETECT-02 | 03-00, 03-01, 03-04 | Real-time watcher detects unlocks from legitimate Steam (binary VDF parser of `<SteamPath>\appcache\stats\UserGameStats_<userid>_<appid>.bin`) | SATISFIED | `vdf_binary.rs::parse_binary_vdf` (binary VDF reader); `steam_legit.rs::SteamLegitAdapter` (full impl); `paths.rs::DiscoveredPaths::steam_legit_appcache_stats`; `lib.rs:201-205` wiring; `integration_phase3.rs::sc1_steam_legit_emits_event_synchronously` + `sc3_supplement_real_three_source_endtoend` + `sc4_lib_run_constructs_all_four_adapters` |
| DETECT-03 | 03-00, 03-02, 03-04 | Real-time watcher detects unlocks from CreamAPI per-appid output | SATISFIED | `cream_api.rs::parse_creamapi_state` + `CreamApiAdapter`; `paths.rs::DiscoveredPaths::cream_api_appid_dirs`; `lib.rs:206-209` wiring; `integration_phase3.rs::sc2_cream_api_and_sse_paths_auto_discovered` + `sc3_three_source_simultaneous_unlock_collapses_to_one_popup` + `sc3_supplement_real_three_source_endtoend` |
| DETECT-04 | 03-00, 03-03, 03-04 | Real-time watcher detects unlocks from SmartSteamEmu per-persona output | SATISFIED | `sse.rs::parse_sse_stats` + `SseAdapter` + `build_crc_reverse_map`; `paths.rs::DiscoveredPaths::sse_appid_dirs`; `lib.rs:210-213` wiring; `integration_phase3.rs::sc2_cream_api_and_sse_paths_auto_discovered` + `sc3_three_source_simultaneous_unlock_collapses_to_one_popup` + `sc3_supplement_real_three_source_endtoend` |

REQUIREMENTS.md traceability table (lines 96-99) marks DETECT-02, DETECT-03, DETECT-04 as Complete in Phase 3. No orphaned requirements — REQUIREMENTS.md maps Phase 3 to exactly DETECT-02/03/04 and all three are claimed by Phase 3 plan frontmatter.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `tests/integration_phase3.rs:177` | Test renamed from `sc1_steam_legit_unlock_fires_within_one_second` to `sc1_steam_legit_emits_event_synchronously` (WR-05) | Latency assertion DROPPED | Info | Documented in REVIEW.md WR-05; the actual <1s SLA is not asserted by any automated test. Surfaced as human_needed item #1. |
| `tests/integration_phase3.rs::sc1_*` | n/a | Bypasses notify-debouncer-full by calling `adapter.on_file_changed` directly | Info | Acknowledged in test docstring lines 167-175 ("notify-debouncer-full integration is already covered by Phase 1's `watcher_core` integration tests + the SC3 test below"). Architecturally sound but reduces SC1's coverage of the real production path. |
| `sources/cream_api.rs::parse_creamapi_state` | line 100-115 | Single `#` lines treated as data, only `###` triple-hash + `;` recognized as comments | Info | Documented in REVIEW.md IN-01 (info finding, deferred). No active impact. |
| `sources/sse.rs::seed_baseline` | lines 289-298 | Lock dance acquires/releases baseline per record | Info | REVIEW.md IN-06 deferred performance optimization. No correctness impact. |
| `sources/steam_legit.rs::discover_paths` | lines 69-85 | Registry-found user_ids skip the filename fallback (no union) | Info | REVIEW.md IN-04 deferred. Could miss legitimate events for user_ids not in registry but present in filenames. |

No blockers, no warnings — all critical (CR-01, CR-02) and warning (WR-01..WR-08) findings from the code review were FIXED in commits c6942a1..0f9547c per REVIEW.md "Fixes Applied" table. Six info findings remain documented for follow-up but do not block phase completion.

### Human Verification Required

#### 1. ROADMAP SC#1 — Full-pipeline 1s latency on a real Steam install

**Test:** Install Hallmark on a Windows machine with a real Steam library and at least one game with achievements. Launch a game, unlock an achievement (or trigger a stat change that should unlock one). Time from in-game unlock to popup appearing.

**Expected:** Hallmark popup appears within 1 second of the achievement unlocking, identical visual + audio quality to a Goldberg unlock popup. No manual path configuration required.

**Why human:** The automated SC1 test (`sc1_steam_legit_emits_event_synchronously`) bypasses notify-debouncer-full entirely (per WR-05 fix), and uses synthetic 12-byte UserGameStats fixtures rather than real Steam files. The production pipeline's full latency budget — file write detection + 500ms debounce + binary VDF parse + diff + dedup + popup queue + audio + animation — is not measured end-to-end against a real Steam unlock. Requires human stopwatch verification on actual hardware.

#### 2. ROADMAP SC#2 — Real-world auto-discovery on user machines

**Test:** On a Windows machine that has CreamAPI installed at `%APPDATA%\CreamAPI\<some_appid>\stats\CreamAPI.Achievements.cfg` AND/OR SmartSteamEmu installed at `%APPDATA%\SmartSteamEmu\<some_appid>\stats.bin`, launch Hallmark with no environment overrides set. Watch the startup tracing logs for `discovery: CreamAPI appid dir` and `discovery: SmartSteamEmu appid dir` lines naming each install.

**Expected:** Each real install is detected and logged at startup. Subsequent achievement unlocks in those emulator-backed games fire the same Hallmark popup as Goldberg unlocks.

**Why human:** Automated SC2 test only verifies discovery against fixture trees via `HALLMARK_CREAMAPI_ROOT_OVERRIDE` / `HALLMARK_SSE_ROOT_OVERRIDE`. The default `dirs::data_dir()`-derived path resolution against the user's real `%APPDATA%` is not exercised by any automated test. A user with real CreamAPI / SSE installs is needed.

#### 3. ROADMAP SC#3 — Concurrent multi-emulator real-world scenario

**Test:** On a machine where a single game is somehow being instrumented by both legit Steam (in `appcache\stats`) AND a Goldberg or CreamAPI overlay simultaneously (unusual but real-world), trigger an achievement unlock and observe the popup count.

**Expected:** Exactly one popup fires per logical unlock, not two or three.

**Why human:** Automated `sc3_three_source_simultaneous_unlock_collapses_to_one_popup` uses MockAdapters firing the same synthetic payload, and `sc3_supplement_real_three_source_endtoend` uses real adapters with synthetic file fixtures. Neither covers the actual concurrent legit + emulator setup against a real game. Optional spot-check; the architectural dedup invariant is proven, but real-world timing windows could differ.

#### 4. Popup quality parity across all 4 source kinds

**Test:** Trigger achievement unlocks from each source kind (Goldberg, SteamLegit, CreamApi, SmartSteamEmu) on a machine where Hallmark is running, and visually + audibly compare the popups.

**Expected:** All 4 source kinds produce visually + audibly identical popups (icon, title, description, animation, signature sound, tier styling).

**Why human:** Automated tests verify only that events flow through the same RawUnlockEvent pipeline. The downstream popup_queue + popup overlay + audio dispatcher are source-agnostic by design (Phase 2), but visual + audible parity is a UX claim only humans can confirm.

### Gaps Summary

The architectural goal is fully delivered:

- 4-adapter pipeline ships in production (lib.rs:195-220).
- Three new adapters (SteamLegit, CreamApi, Sse) implement the full SourceAdapter contract with parser + baseline + dedup.
- DETECT-02, DETECT-03, DETECT-04 are functionally satisfied with passing automated tests.
- Cross-source dedup proven to generalize from 2 to N adapters via SC3 (3 mock adapters → 1 event + 1 row) and SC3-supplement (3 real adapters with synthetic fixtures → 1 event for app_id).
- Phase 1 + lib tests pass without regression (5/5 + 132/132 + 5/5 = 142 tests passing across the 3 test surfaces).
- The plan-level code review found 2 critical + 8 warning issues; all 10 were fixed in commits c6942a1..0f9547c per REVIEW.md.

The verification status is `human_needed` because the three ROADMAP success criteria reference real-world conditions ("Steam client running," "auto-detected by path discovery on the user machine," "running alongside a real game") that no automated test can fully exercise. The most important gap is SC#1's 1-second latency claim — the original automated assertion was intentionally dropped during code review (WR-05) because the test bypassed the production debouncer; the SLA is therefore unverified. Phase 3 should not block Phase 4 (Polish & Distribution), but the four human verification items above should be tracked for Phase 4 / pre-release UAT.

---

_Verified: 2026-05-09T13:05:00Z_
_Verifier: Claude (gsd-verifier)_
