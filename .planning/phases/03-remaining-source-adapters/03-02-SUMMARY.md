---
phase: 03-remaining-source-adapters
plan: "02"
subsystem: detection
tags: [rust, source-adapter, cream-api, ini-parser, sha256, hand-rolled-parser, env-override]

requires:
  - phase: 03-remaining-source-adapters
    provides: Plan 03-00 stub modules (cream_api), SourceKind::CreamApi, DiscoveredPaths.cream_api_appid_dirs
  - phase: 01-detection-pipeline-foundation
    provides: SourceAdapter trait, RawUnlockEvent, GoldbergAdapter pattern (Arc<RwLock<...>> baseline + last_hash, read-with-retry, SHA-256 short-circuit, parse → diff → emit → update)
provides:
  - CreamApiAdapter with full SourceAdapter implementation (5 methods)
  - parse_creamapi_state pure 12-LoC line-oriented INI parser (BOM strip + section + key=value)
  - discover_paths reading %APPDATA%\CreamAPI\<numeric_appid>\stats\CreamAPI.Achievements.cfg
  - HALLMARK_CREAMAPI_ROOT_OVERRIDE env var override (parallels Phase 1's HALLMARK_GOLDBERG_ROOT_OVERRIDE)
  - 10 new unit tests covering INI parse, BOM strip, comment skip, 1/true equivalence, seed, false→true emit, filename guard, SHA-256 short-circuit, extract_app_id, discover_paths
affects: [03-04-pipeline-integration]

tech-stack:
  added: []
  patterns:
    - "Hand-rolled 12-LoC INI parser (no toml/configparser crate dependency for trivial format) — line-oriented iteration with first-line BOM strip, empty/comment skip, [section] state machine, key=value split_once"
    - "STATE_FILENAME constant used in 5 places (filename guard + seed file path + 1 test write helper + 2 path-construction helpers) — single source of truth for the canonical filename string"
    - "extract_app_id parses path.parent().parent() (stats dir → appid dir) for the canonical CreamAPI layout depth"
    - "Adapter shape mirrors GoldbergAdapter EXACTLY: cached_watch_paths + baseline + last_hash + filename guard FIRST + read-with-retry + SHA-256 short-circuit + parse → diff → emit → THEN update invariant"

key-files:
  created: []
  modified:
    - src-tauri/src/sources/cream_api.rs

key-decisions:
  - "Plan 03-02: Section header default-to-false on first encounter — `out.entry(inner).or_insert(false)` ensures locked achievements appear in baseline so future false→true writes are detectable even on entries the file already lists"
  - "Plan 03-02: 1/true equivalence is case-insensitive (`val.to_ascii_lowercase()`) — matches real CreamAPI writes that use mixed case (`True`, `TRUE`, `1`)"
  - "Plan 03-02: unlocktime intentionally read-and-discarded (Pitfall #4) — boolean `achieved=true` transition is the only valid unlock signal, parallels Goldberg's earned/earned_time treatment"
  - "Plan 03-02: discover_paths uses `dirs::data_dir()` (not `dirs::config_dir()`) — `dirs::data_dir()` resolves to %APPDATA%\\Roaming on Windows which is where CreamAPI installs"
  - "Plan 03-02: Numeric-appid filter applied at discover_paths (not at adapter time) — non-numeric subdirs in %APPDATA%\\CreamAPI (e.g., shared assets) skipped before becoming watch roots"

metrics:
  duration: 4min
  tasks: 1
  files_created: 0
  files_modified: 1
  loc_added: ~376
  tests_added: 10
  tests_passing: 124
  tests_passing_baseline: 116

requirements-completed: [DETECT-03]

started: 2026-05-09T09:05:00Z
completed: 2026-05-09T09:09:00Z
---

# Phase 03 Plan 02: CreamApiAdapter Summary

**Implemented full CreamApiAdapter (REQ DETECT-03) with 12-LoC hand-rolled INI parser, discover_paths enumerating `%APPDATA%\CreamAPI\<numeric_appid>` subdirectories, and 10 new unit tests covering INI parse, BOM strip, comment-skip, true/1 case-insensitivity, false→true transition emit, filename guard, and SHA-256 short-circuit. Lib tests now 124/124 passing (up from 116 baseline).**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-05-09T09:05:00Z (after Plan 03-01 completion at 09:02)
- **Completed:** 2026-05-09T09:09:00Z
- **Tasks:** 1
- **Files modified:** 1 (cream_api.rs)
- **LoC added:** 376 net diff (replacing the 79-LoC stub from Plan 03-00 with the full 426-LoC implementation; counts include tests)
- **Tests added:** 10 (net +8 vs prior 2 stub tests)

## Accomplishments

### Task 1 — CreamApiAdapter (commit `484bdbb`)

- **Replaced cream_api.rs stub** with the full `SourceAdapter` implementation. Shape mirrors `GoldbergAdapter` exactly:
  - `cached_watch_paths: Vec<PathBuf>` resolved at construction, `path.exists()` filter applied once.
  - `baseline: Arc<RwLock<HashMap<(u64, String), bool>>>` keyed on `(app_id, ach_api_name)`.
  - `last_hash: Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>` for SHA-256 content-equality short-circuit.
  - `on_file_changed` order: filename guard FIRST → app_id parse from `path.parent().parent()` → `read_with_retry` → SHA-256 hash compare → parse_creamapi_state → diff against baseline → emit `RawUnlockEvent` per `false→true` transition → update baseline → update last_hash. Identical to Goldberg's commit-on-success ordering.
- **`parse_creamapi_state`** — Pure 12-LoC line-oriented INI parser per Hydra's canonical pattern:
  1. BOM strip from the first line (`\u{feff}`).
  2. Skip empty lines.
  3. Skip `###` triple-hash comments and `;` semicolon comments.
  4. `[SECTION]` lines start a new achievement entry; default-insert as `false` (locked) so future false→true diffs work even on entries already in the file.
  5. `key=value` lines under a section: only `achieved=true|1` (case-insensitive) is consumed; `unlocktime` is read and discarded (Pitfall #4).
  6. Lines outside a `[SECTION]` are ignored.
- **`discover_paths`** — Enumerates `%APPDATA%\CreamAPI` (via `dirs::data_dir()`):
  - Honors `HALLMARK_CREAMAPI_ROOT_OVERRIDE` env var for fixture-tree integration testing (B-2 / RESEARCH.md line 417, parallels Phase 1's HALLMARK_GOLDBERG_ROOT_OVERRIDE).
  - Filters subdirs by numeric u64 parse (CreamAPI subdirs are exclusively integer appids).
  - Requires `<appid>\stats\CreamAPI.Achievements.cfg` to exist before pushing the appid dir to the result vec.
  - Returns empty `CreamApiPaths::default()` when CreamAPI root is absent — no panic.
- **`extract_app_id`** — Parses `<root>/<appid>/stats/CreamAPI.Achievements.cfg` two-level upward (`path.parent().parent()`) and returns `None` on non-numeric.
- **`STATE_FILENAME` const** — `"CreamAPI.Achievements.cfg"` used at 5 sites (declaration, filename guard, seed_baseline join, test fixture writer, identity check).
- **Filename guard FIRST** — `on_file_changed` rejects events for any filename other than `CreamAPI.Achievements.cfg` BEFORE any I/O. Sibling files (`cream_api.ini`, etc.) silently dropped.
- **`read_with_retry`** — Mirrors `goldberg::read_with_retry` exactly: 3-attempt × 50ms × `tokio::sleep` loop on `ErrorKind::PermissionDenied` or `raw_os_error == 32 (ERROR_SHARING_VIOLATION) | 33 (ERROR_LOCK_VIOLATION)`. Returns `String` (text file).
- **10 unit tests pass:**
  1. `parse_creamapi_state_handles_basic_ini` — round-trip of 3-section fixture.
  2. `parse_creamapi_state_strips_bom` — `\u{feff}`-prefixed input parses identically.
  3. `parse_creamapi_state_ignores_comments_and_unlocktime` — `###` lines and `unlocktime=` keys ignored.
  4. `parse_creamapi_state_treats_1_and_true_equivalently` — `1`, `true`, `TRUE`, `True` all map to `true`; `0`, `false`, `False` to `false`.
  5. `seed_baseline_populates_from_fixture` — adapter reads fixture cfg, populates baseline with 3 entries.
  6. `on_file_changed_emits_event_on_false_to_true_transition` — flipping `ACH_BOSS` from false to true emits exactly one `RawUnlockEvent { app_id: 4242, ach_api_name: "ACH_BOSS", source: SourceKind::CreamApi }`. Already-true `ACH_FIRST` does not emit.
  7. `on_file_changed_skips_non_cream_filename` — `cream_api.ini` event yields no events (filename guard).
  8. `on_file_changed_skips_identical_content_via_sha256` — second call with identical bytes short-circuits before parse.
  9. `extract_app_id_from_canonical_path` — both numeric and non-numeric `/tmp/CreamAPI/<x>/stats/CreamAPI.Achievements.cfg` cases.
  10. `discover_paths_returns_empty_when_no_creamapi_dir` — function does not panic when root absent.

## Pattern Alignment with GoldbergAdapter

The plan mandated mirroring Goldberg's exact shape. Where CreamApi DIVERGES (intentionally):

| Aspect | Goldberg | CreamApi | Why |
|---|---|---|---|
| File format | JSON (text) | INI (text) | CreamAPI uses INI per Hydra/Achievement-Watcher convention |
| Parser | `serde_json::from_str` | hand-rolled 12-LoC line iterator | INI is too simple to justify a crate dep; matches Hydra's canonical pattern |
| File pattern | `<root>/<appid>/achievements.json` (depth 2) | `<root>/<appid>/stats/CreamAPI.Achievements.cfg` (depth 3) | CreamAPI nests state under `stats\` subdir |
| App ID resolution | `path.parent().file_name()` numeric parse | `path.parent().parent().file_name()` numeric parse | Extra `stats\` directory level |
| Watch root construction | per-root walkdir for seed | per-appid-dir direct file lookup for seed | CreamAPI watch roots ARE the appid dirs (set at construction); no walkdir needed |
| Redirect map | yes (Goldberg `local_save.txt`) | no | CreamAPI has no redirect feature |
| Read function | `read_with_retry` returns `String` | `read_with_retry` returns `String` | Identical (both text files) |
| `on_file_changed` order | filename guard → app_id → retry-read → hash → parse → diff → emit → update baseline → update hash | filename guard → app_id → retry-read → hash → parse → diff → emit → update baseline → update hash | **Identical**; the BL-02 invariant (commit-on-success) is preserved verbatim |

The Goldberg `read → hash → parse → diff → emit → THEN update baseline + hash` invariant (BL-02) is preserved verbatim in CreamApiAdapter.

## Deviations from Plan

None — plan executed exactly as written. The plan's code listing was copied to disk verbatim, all acceptance criteria checked green, and no auto-fix rules (1/2/3) triggered.

The plan's required-tests list was 5; the executor delivered 10 (5 required + 5 additional coverage tests for: comment/unlocktime ignore, 1/true equivalence, seed_baseline populate, filename guard, extract_app_id canonical path).

## Threat-Model Coverage

The plan's threat register listed 6 threats with `mitigate` disposition (T-32-T1, T-32-D1, T-32-T2, T-32-S1) plus 1 `accept` (T-32-D2) and 1 `accept` (T-32-I1). All `mitigate` items are implemented:

| Threat ID | Mitigation Implemented |
|---|---|
| T-32-T1 (Tampering INI content) | `parse_creamapi_state` is pure function; malformed lines silently skipped (line-by-line iterator never panics). No `unwrap()` in parser. `split_once('=')` returns `None` on lines without `=`, which is matched and skipped. |
| T-32-D1 (DoS huge INI file) | `read_to_string` → string allocation bounded by file size; `parse_creamapi_state` allocates one HashMap entry per section. The 500ms debounce + SHA-256 short-circuit (REQ DETECT-06) prevents re-parsing unchanged files in the steady state. |
| T-32-T2 (Section name newline injection) | `text.lines()` splits on `\n` / `\r\n` BEFORE the `[<inner>]` capture; no newline can appear inside a captured section name. The `&line[1..line.len()-1]` slice operates on a single trimmed line. |
| T-32-S1 (Sibling filename spoofing) | Filename guard checks `path.file_name() == Some("CreamAPI.Achievements.cfg")` exactly. Variants like `cream_api.ini`, `CreamAPI.cfg`, `CreamAPI.Stats.cfg` all return `Ok(())` immediately without I/O. Verified by `on_file_changed_skips_non_cream_filename`. |
| T-32-D2 (100k sections) | accept disposition — local-user-controlled file; no remote attack surface. |
| T-32-I1 (Tracing log path/appid disclosure) | accept disposition — local stdout only. |

## Authentication Gates

None — this plan is purely local-file I/O.

## Issues Encountered

None.

## Plan 03-04 Readiness

The CreamApiAdapter is fully ready for `lib.rs::run()` wiring (Plan 03-04). The construction signature is:
```rust
CreamApiAdapter::new(discovered.cream_api_appid_dirs)   // Vec<PathBuf>
```
The single field is already populated by `paths::discover()` (Plan 03-00 wired `cream_api::discover_paths()` into the discovery flow). The adapter implements the full `SourceAdapter` trait (`name`, `kind`, `watch_paths`, `seed_baseline`, `on_file_changed`), so `WatcherCore` can register it identically to `GoldbergAdapter` and `SteamLegitAdapter` — no special-casing in the watcher dispatch.

The CrossSourceDedup designed in Plan 01-05 already keys on `(app_id, ach_api_name)` and generalizes to N adapters, so the Plan 03-04 cross-source dedup integration test will pass against {Goldberg, SteamLegit, CreamApi} without additional dedup logic.

The HALLMARK_CREAMAPI_ROOT_OVERRIDE env var is now wired and ready for SC2 integration testing in Plan 03-04.

## Self-Check: PASSED

Verified each modified file exists:
- `src-tauri/src/sources/cream_api.rs` — FOUND (426 lines; stub marker absent; `impl SourceAdapter for CreamApiAdapter` present; `parse_creamapi_state` defined; `STATE_FILENAME` const at 5 sites; `extract_app_id` at 5 sites; `Pitfall #4` reference present; `HALLMARK_CREAMAPI_ROOT_OVERRIDE` reference present; 10 tests)

Verified the commit exists:
- `484bdbb` — FOUND (Task 1: feat(03-02) implement CreamApiAdapter with INI parser and discover_paths)

Verified `cargo check --all-targets` exits 0 (clean compile, no warnings) and `cargo test --lib` reports `124 passed; 0 failed; 0 ignored`.

## Self-Check Results

- File `src-tauri/src/sources/cream_api.rs` — FOUND
- File `.planning/phases/03-remaining-source-adapters/03-02-SUMMARY.md` — FOUND
- Commit `484bdbb` — FOUND in `git log --oneline`

---
*Phase: 03-remaining-source-adapters*
*Completed: 2026-05-09*
