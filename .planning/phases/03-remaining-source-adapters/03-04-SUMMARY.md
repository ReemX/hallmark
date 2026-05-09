---
phase: 03-remaining-source-adapters
plan: "04"
subsystem: detection
tags: [rust, integration-test, lib-rs-wiring, cross-source-dedup, mock-adapter, env-guard, capstone]

requires:
  - phase: 03-remaining-source-adapters
    provides: Plan 03-00 stub modules + DiscoveredPaths fields + env-var override hooks; Plan 03-01 SteamLegitAdapter::new(Option<PathBuf>, Vec<u64>); Plan 03-02 CreamApiAdapter::new(Vec<PathBuf>) + HALLMARK_CREAMAPI_ROOT_OVERRIDE; Plan 03-03 SseAdapter::new(Vec<PathBuf>) + HALLMARK_SSE_ROOT_OVERRIDE
  - phase: 01-detection-pipeline-foundation
    provides: run_watcher(Vec<Arc<dyn SourceAdapter>>, mpsc::Sender<RawUnlockEvent>) -> anyhow::Result<()>; run_pipeline(...) -> anyhow::Result<()>; CrossSourceDedup keyed on (app_id, ach_api_name) with 10s TTL; SQLite UNIQUE INDEX idx_unlock_dedup belt-and-suspenders
provides:
  - lib.rs::run() setup() constructing the 4-adapter pipeline (Goldberg, SteamLegit, CreamApi, Sse) wired into the existing run_watcher + run_pipeline topology
  - Phase 3 integration test file at src-tauri/tests/integration_phase3.rs containing 5 #[tokio::test]s proving all 3 ROADMAP success criteria
  - End-to-end verification that CrossSourceDedup generalizes from 2 to N source adapters without code change (architectural assumption from Phase 1 Plan 01-05 confirmed empirically)
  - Reusable EnvGuard RAII helper for env-var override testing (HALLMARK_CREAMAPI_ROOT_OVERRIDE / HALLMARK_SSE_ROOT_OVERRIDE)
affects: [phase-04-polish-distribution]

tech-stack:
  added: []
  patterns:
    - "EnvGuard RAII pattern for tests that mutate process env: stores prev OsString, sets new value on construction, restores or removes on Drop. Avoids env-var leakage across test runs in the same process."
    - "MockAdapter pattern (Phase 1 SC4 analog) for cross-source dedup tests: file-event-driven adapter parsing `<app_id>,<ach_api_name>` from a trigger.txt write — isolates dedup behaviour from per-adapter parser complexity."
    - "Direct on_file_changed call in SC1 (DETECT-02 verification): bypasses notify-debouncer-full to deterministically prove the adapter's parse → diff → emit pipeline produces a SteamLegit RawUnlockEvent in <1s; debouncer integration is already covered by Phase 1 watcher_core tests + SC3 below."
    - "Synthetic binary VDF schema fixtures: hand-encoded 0x00 root Object + numeric stat_slot Object + 0x01 String name entry + 0x08 close — robust against extract_schema_mapping's fallback (root_obj when no numeric-appid child) so test stays decoupled from internal walk strategy."

key-files:
  created:
    - src-tauri/tests/integration_phase3.rs
  modified: []

key-decisions:
  - "Plan 03-04: SC1 calls adapter.on_file_changed DIRECTLY rather than relying on notify-debouncer-full (B-1 fix). Deterministic across CI/local; debouncer integration is covered by SC3 + Phase 1 watcher_core tests."
  - "Plan 03-04: SC3 (headline test) uses 3 MockAdapter instances rather than the real adapters — isolates the architectural assertion (CrossSourceDedup collapses N simultaneous emits to 1) from per-adapter parser complexity. Real-adapter coverage lives in SC3-supplement."
  - "Plan 03-04: SC3-supplement accepts BOTH the schema-resolved api_name (ACH_SC3_SHARED) AND the SSE placeholder format <crc:0x...> — whichever source wins the dedup race determines the canonical name; the dedup invariant holds regardless because the same CRC produces the same placeholder string deterministically."
  - "Plan 03-04: spawn_pipeline returns watcher_handle as JoinHandle<anyhow::Result<()>> AND pipeline_handle as JoinHandle<anyhow::Result<()>> (corrected from the plan's draft type). Matches the actual run_watcher / run_pipeline signatures."
  - "Plan 03-04: Goldberg companion file written at the real %APPDATA%\\GSE Saves\\<app_id>\\achievements.json in SC3-supplement (no env-var override exists for this v1) — pre-existence flag drives cleanup so the test does not delete a user's real Goldberg state."

metrics:
  duration: 6min
  tasks: 2
  files_created: 1
  files_modified: 0
  loc_added: ~803
  tests_added: 5
  tests_passing_lib: 131
  tests_passing_integration_phase1: 5
  tests_passing_integration_phase3: 5
  total_phase_3_tests: 5

requirements-completed: [DETECT-02, DETECT-03, DETECT-04]

started: 2026-05-09T09:21:00Z
completed: 2026-05-09T09:27:00Z
---

# Phase 03 Plan 04: Pipeline Integration & Phase 3 Capstone Summary

**Wired the 4-adapter pipeline into `lib.rs::run()` and added five integration tests verifying all three ROADMAP Phase 3 success criteria pass end-to-end. The headline 3-source dedup test (SC3) proves CrossSourceDedup collapses three near-simultaneous file-event-driven adapter emits into exactly ONE event at the sink + ONE row in `unlock_history`. Phase 3 is now CLOSED.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-05-09T09:21:00Z (after Plan 03-03 completion at 09:19)
- **Completed:** 2026-05-09T09:27:00Z
- **Tasks:** 2
- **Files created:** 1 (integration_phase3.rs, 803 LoC)
- **Files modified:** 0 (Task 1 — lib.rs wiring — already committed in 5a44fe4)
- **Tests added:** 5 (SC1, SC2, SC3, SC3-supplement, SC4)

## Accomplishments

### Task 1 — lib.rs wiring (commit `5a44fe4`, recorded ahead of plan execution)

The `lib.rs::run()` setup() closure now constructs the full 4-adapter pipeline:

```rust
let goldberg_adapter:    Arc<dyn SourceAdapter> = Arc::new(GoldbergAdapter::new(goldberg_paths.clone(), goldberg_map.clone()));
let steam_legit_adapter: Arc<dyn SourceAdapter> = Arc::new(SteamLegitAdapter::new(discovery.steam_legit_appcache_stats.clone(), discovery.steam_legit_user_ids.clone()));
let cream_api_adapter:   Arc<dyn SourceAdapter> = Arc::new(CreamApiAdapter::new(discovery.cream_api_appid_dirs.clone()));
let sse_adapter:         Arc<dyn SourceAdapter> = Arc::new(SseAdapter::new(discovery.sse_appid_dirs.clone()));
let adapters = vec![goldberg_adapter, steam_legit_adapter, cream_api_adapter, sse_adapter];
tracing::info!(adapter_count = adapters.len(), "Phase 3: 4-adapter pipeline configured");
```

The Vec is then handed to `watcher::run_watcher` exactly as before — no new dispatch code; CrossSourceDedup (Plan 01-05) already keys on `(app_id, ach_api_name)` and generalizes to N adapters by design.

### Task 2 — Phase 3 integration tests (commit `6fc18db`)

Created `src-tauri/tests/integration_phase3.rs` (803 LoC) with five `#[tokio::test]`s:

| Test | ROADMAP SC | What it proves |
|---|---|---|
| `sc1_steam_legit_unlock_fires_within_one_second` | SC#1 | Synthetic binary VDF state (`cache.<1>.data=0` → `=1`) + direct `adapter.on_file_changed` call yields `RawUnlockEvent { source: SteamLegit }` with non-empty `ach_api_name` (placeholder OR schema-resolved) in <1s. Repeat call with identical content short-circuits via SHA-256 (no duplicate). |
| `sc2_cream_api_and_sse_paths_auto_discovered` | SC#2 | `EnvGuard` redirects `HALLMARK_CREAMAPI_ROOT_OVERRIDE` + `HALLMARK_SSE_ROOT_OVERRIDE` to fixture trees with populated `4242/stats/CreamAPI.Achievements.cfg` + `4242/stats.bin`. `discover_paths()` returns both `4242` dirs **with no manual configuration**. The full pipeline drives both adapters, both emit events for `app_id = 4242` (one `CreamApi`, one `SmartSteamEmu`). |
| `sc3_three_source_simultaneous_unlock_collapses_to_one_popup` | SC#3 (headline) | Three `MockAdapter` instances (kinds: `SteamLegit`, `CreamApi`, `SmartSteamEmu`) write `trigger.txt` with the same `"777,ACH_TRIPLE_OBSERVED"` payload. `CrossSourceDedup` collapses into **EXACTLY 1 event** at the sink + **EXACTLY 1 row** in `unlock_history`. |
| `sc3_supplement_real_three_source_endtoend` | SC#3 (B-3 fix) | Same dedup property with REAL `SteamLegitAdapter` + `CreamApiAdapter` + `SseAdapter` against synthetic `UserGameStats_*.bin` + `UserGameStatsSchema_*.bin` + `CreamAPI.Achievements.cfg` + `stats.bin` fixtures. Goldberg companion file at real `%APPDATA%\GSE Saves\9999\achievements.json` enables CRC reverse-lookup; cleanup respects pre-existing user files via `goldberg_pre_existed` flag. |
| `sc4_lib_run_constructs_all_four_adapters` | (production-shape proof) | Reconstructs the same `Vec<Arc<dyn SourceAdapter>>` lib.rs::run() builds; asserts `len == 4`, distinct `name()`, distinct `kind()`. Proves the production wiring matches the integration-tested topology. |

All five tests pass: `test result: ok. 5 passed; 0 failed; 0 ignored`.

## ROADMAP Phase 3 Success Criteria Coverage

| ROADMAP SC | Criterion | Verified by | Status |
|---|---|---|---|
| #1 | Steam-legit unlock fires popup within 1s, identical quality to Goldberg, no manual config | `sc1_steam_legit_unlock_fires_within_one_second` (DETECT-02 emission proof) | PASS |
| #2 | CreamAPI + SSE installs auto-detected via path discovery; unlocks fire same premium popup | `sc2_cream_api_and_sse_paths_auto_discovered` (auto-discover + pipeline emit) | PASS |
| #3 | Multi-adapter same logical unlock → exactly one popup (not two or three) | `sc3_three_source_simultaneous_unlock_collapses_to_one_popup` (3 mock adapters → 1 event) **+** `sc3_supplement_real_three_source_endtoend` (3 real adapters → 1 event) | PASS |

## REQ DETECT Coverage

| Requirement | Test verifying end-to-end behaviour |
|---|---|
| DETECT-02 (Steam-legit binary VDF) | `sc1_steam_legit_unlock_fires_within_one_second`, `sc3_supplement_real_three_source_endtoend`, `sc4_lib_run_constructs_all_four_adapters` |
| DETECT-03 (CreamAPI INI per-appid) | `sc2_cream_api_and_sse_paths_auto_discovered`, `sc3_three_source_simultaneous_unlock_collapses_to_one_popup`, `sc3_supplement_real_three_source_endtoend`, `sc4_lib_run_constructs_all_four_adapters` |
| DETECT-04 (SmartSteamEmu stats.bin) | `sc2_cream_api_and_sse_paths_auto_discovered`, `sc3_three_source_simultaneous_unlock_collapses_to_one_popup`, `sc3_supplement_real_three_source_endtoend`, `sc4_lib_run_constructs_all_four_adapters` |

DETECT-02 / DETECT-03 / DETECT-04 are now functionally satisfied AND covered by automated regression tests.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Corrected `pipeline_handle` JoinHandle type from `JoinHandle<()>` to `JoinHandle<anyhow::Result<()>>`**
- **Found during:** Task 2 first compile attempt (`cargo test --no-run`).
- **Issue:** The plan's `spawn_pipeline` listing typed `pipeline_handle` as `JoinHandle<()>` based on an outdated assumption that `run_pipeline` returns `()`. The actual signature is `run_pipeline(...) -> anyhow::Result<()>` (watcher/mod.rs:350).
- **Fix:** Changed `pipeline_handle: tokio::task::JoinHandle<()>` to `tokio::task::JoinHandle<anyhow::Result<()>>` and removed the unnecessary `async move { run_pipeline(...).await; }` wrapper — `tokio::spawn(run_pipeline(...))` works directly.
- **Files modified:** `src-tauri/tests/integration_phase3.rs` (test helper only).
- **Verification:** `cargo test --test integration_phase3 --no-run` exits 0; all 5 tests pass.
- **Committed in:** `6fc18db` (Task 2 commit; the corrected version went straight into the file before commit, no separate fix-up commit needed).

**Total deviations:** 1 auto-fixed (Rule 3 — Blocking). No Rule 1/2/4 triggers.

## Verification Run Results

```
cargo check --all-targets          → exits 0 (no warnings)
cargo test --lib --no-fail-fast    → 131 passed; 0 failed; 0 ignored (no regressions vs Plan 03-03 baseline)
cargo test --test integration_phase1 → 5 passed; 0 failed; 0 ignored (Phase 1 SCs unchanged)
cargo test --test integration_phase3 → 5 passed; 0 failed; 0 ignored (NEW — all Phase 3 SCs pass)
```

## Threat-Model Coverage

The plan's threat register listed five threats; dispositions were `accept` (T-34-T1, T-34-D1, T-34-I1, T-34-R1) and `mitigate` (T-34-S1).

| Threat ID | Mitigation Verified |
|---|---|
| T-34-T1 (adapter ordering affects dispatch) | `accept` — Phase 1's WatcherCore dispatches to ALL matching adapters; CrossSourceDedup keys on `(app_id, ach_api_name)` not adapter index. SC3 verifies dedup is order-independent (any of the 3 adapters can win the race). |
| T-34-D1 (4 × N watch paths exceed debouncer cap) | `accept` — practical limit ~200 paths well within notify-debouncer-full's documented capacity; 500ms debounce + per-adapter SHA-256 short-circuit bound burst behaviour. |
| T-34-S1 (overlapping watch paths) | `mitigate` — Phase 1 WR-09 logs error at startup; CrossSourceDedup catches resulting duplicates. SC3 + SC3-supplement verify dedup is robust to simultaneous arrivals. |
| T-34-I1 (aggregated tracing logs) | `accept` — local stdout only. |
| T-34-R1 (10s TTL exceeded → repudiation) | `accept` — sub-second cross-adapter simultaneity in practice; SQLite UNIQUE INDEX is the belt-and-suspenders second layer (verified by SC3 row-count assertion). |

## Authentication Gates

None — this plan is purely local-file/test I/O.

## Issues Encountered

- Initial cargo build flagged the `pipeline_handle: JoinHandle<()>` mismatch (the plan's draft type). Resolved as documented above. No other issues.

## Phase 3 Closeout

**Phase 3 (Remaining Source Adapters) is now COMPLETE.** All 5 plans landed:

- `03-00` — Pre-flight spike (DETECT-02 path correction + Cargo deps + 4 stub modules + DiscoveredPaths extension).
- `03-01` — SteamLegitAdapter (binary VDF reader + mtime-cached schema + registry user_id discovery).
- `03-02` — CreamApiAdapter (12-LoC INI parser + numeric-appid discovery + env-var override).
- `03-03` — SseAdapter (24-byte record parser + lazy CRC32 reverse-lookup + Goldberg companion harvest).
- `03-04` — Pipeline integration + 3-source dedup capstone test (this plan).

The 4-adapter pipeline ships in production. CrossSourceDedup generalization from 2 to N adapters is empirically proven. **REQ DETECT-02 / DETECT-03 / DETECT-04 are satisfied.**

**Phase 4 (Polish & Distribution) is now UNBLOCKED.**

## Self-Check: PASSED

Verified each created file exists:
- `src-tauri/tests/integration_phase3.rs` — FOUND (803 lines; 5 #[tokio::test] functions; 9 EnvGuard references; 4 HALLMARK_CREAMAPI_ROOT_OVERRIDE references; 4 HALLMARK_SSE_ROOT_OVERRIDE references; rusqlite::params used in SC3 + SC3-supplement; MockAdapter struct + impl SourceAdapter present)

Verified each commit exists:
- `5a44fe4` — FOUND (Task 1: feat(03-04) wire 3 new adapters into lib.rs::run() setup())
- `6fc18db` — FOUND (Task 2: test(03-04) add Phase 3 integration tests verifying ROADMAP success criteria)

Verified test runs:
- `cargo check --all-targets` exits 0
- `cargo test --lib` reports `131 passed; 0 failed; 0 ignored`
- `cargo test --test integration_phase1` reports `5 passed; 0 failed; 0 ignored`
- `cargo test --test integration_phase3` reports `5 passed; 0 failed; 0 ignored`

---
*Phase: 03-remaining-source-adapters*
*Completed: 2026-05-09*
