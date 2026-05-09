---
phase: 03-remaining-source-adapters
plan: "01"
subsystem: detection
tags: [rust, source-adapter, steam-legit, binary-vdf, schema-cache, mtime, sha256, registry]

requires:
  - phase: 03-remaining-source-adapters
    provides: Plan 03-00 stub modules (vdf_binary, steam_legit), Cargo deps (byteorder 1.5), DiscoveredPaths fields, SourceKind::SteamLegit
  - phase: 01-detection-pipeline-foundation
    provides: SourceAdapter trait, RawUnlockEvent, GoldbergAdapter pattern (Arc<RwLock<...>> baseline + last_hash, read-with-retry, SHA-256 short-circuit, parse → diff → emit → update)
provides:
  - Full hand-rolled binary VDF reader (vdf_binary::parse_binary_vdf) handling all 9 documented type tags 0x00..0x08
  - SteamLegitAdapter with mtime-keyed AppSchema cache + per-app schema lookup + state diff
  - Registry-driven user_id enumeration via HKCU\Software\Valve\Steam\Users with filename-fallback
  - Multi-user filter (Pitfall #5) — drops events for non-active Steam accounts
  - Missing-schema fallback (Pitfall #8) — placeholder ach_api_name format steam_stat_<stat>_<bit>
  - 4 real binary fixtures (state + schema for appid 480 Spacewar + appid 2670630)
  - 13 new unit tests (5 vdf_binary + 8 steam_legit)
affects: [03-02-cream-api-adapter, 03-03-sse-adapter, 03-04-pipeline-integration]

tech-stack:
  added: []
  patterns:
    - "Hand-rolled binary parser bounded by recursion depth (16) + C-string length (1024) — defends T-31-D2 + T-31-T1 without external crate dependency for the VDF dialect"
    - "Two-file dependency adapter: state file is the watch trigger, schema file is the lookup, schema is mtime-cached and read on demand at first state event for an app_id"
    - "Schema cache uses Arc<RwLock<HashMap<u64, AppSchema>>> keyed on (loaded_mtime: SystemTime) — only re-parses when filesystem mtime advances"
    - "Filename-guard FIRST in on_file_changed (Goldberg parity) — schema-file events are pre-filtered before any I/O; non-matching filenames return Ok(()) immediately"
    - "Read-bytes-with-retry mirrors goldberg::read_with_retry but for &[u8] (not String) — same Windows ERROR_SHARING_VIOLATION (32) / ERROR_LOCK_VIOLATION (33) handling"

key-files:
  created:
    - src-tauri/tests/fixtures/steam_legit/UserGameStats_132274694_480.bin
    - src-tauri/tests/fixtures/steam_legit/UserGameStatsSchema_480.bin
    - src-tauri/tests/fixtures/steam_legit/UserGameStats_132274694_2670630.bin
    - src-tauri/tests/fixtures/steam_legit/UserGameStatsSchema_2670630.bin
  modified:
    - src-tauri/src/sources/vdf_binary.rs
    - src-tauri/src/sources/steam_legit.rs

key-decisions:
  - "Plan 03-01: Bind extract_state_mapping to plain (data == 1) AND AchievementTimes presence — covers both stat-as-bool and bit-mapped Steam achievement encodings observed in real fixtures"
  - "Plan 03-01: Schema container path-walk is deterministic (numeric-appid key descent then root fallback) — NOT heuristic on root_obj.len() — robust to schema files with extra root metadata"
  - "Plan 03-01: Schema-file watcher events are pre-filtered (returns Ok(()) before any read) — schema is only loaded mtime-on-demand at the next state-file event for that app_id"
  - "Plan 03-01: Missing-schema placeholder format steam_stat_<stat>_<bit> — popup still fires with degraded display (Pitfall #8) instead of silently dropping"
  - "Plan 03-01: discover_paths registry fallback — if HKCU\\Software\\Valve\\Steam\\Users is empty, scan UserGameStats_*.bin filenames in appcache/stats to extract a user_id (covers fresh installs that haven't logged in via Steam yet)"

metrics:
  duration: 8min
  tasks: 2
  files_created: 4
  files_modified: 2
  loc_added_vdf_binary: 224
  loc_added_steam_legit: 629
  tests_added: 13
  tests_passing: 116
  tests_passing_baseline: 106

requirements-completed: [DETECT-02]

started: 2026-05-09T08:54Z
completed: 2026-05-09T09:02Z
---

# Phase 03 Plan 01: SteamLegitAdapter Summary

**Implemented full SteamLegitAdapter (REQ DETECT-02) including a hand-rolled binary VDF reader, mtime-cached schema lookup, registry-driven user_id discovery, and 13 new unit tests with 4 real `UserGameStats*.bin` / `UserGameStatsSchema*.bin` fixtures from the dev machine. lib tests now 116/116 passing (up from 106 baseline).**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-05-09T08:54Z (after Plan 03-00 completion at 08:54)
- **Completed:** 2026-05-09T09:02Z
- **Tasks:** 2
- **Files created:** 4 (binary fixtures)
- **Files modified:** 2 (vdf_binary.rs, steam_legit.rs)
- **LoC added:** ~853 net (224 vdf_binary, 629 steam_legit; counts include tests)
- **Tests added:** 13 (5 vdf_binary + 8 steam_legit; net +10 vs prior stubs)

## Accomplishments

### Task 1 — Binary VDF Reader (commit `23e125c`)

- **Replaced vdf_binary.rs stub** with the full hand-rolled parser handling all 9 documented type tags: `0x00` Object (recursive), `0x01` String (NUL-terminated UTF-8 cstr), `0x02` Int32, `0x03` Float, `0x04` Pointer (skipped), `0x05` WString (UTF-16LE NUL-terminated, skipped), `0x06` Color (skipped), `0x07` UInt64, `0x08` ObjectEnd.
- **Recursion bounded** at depth 16 (`MAX_RECURSION_DEPTH`), well above empirical depth ≤4. C-strings bounded at 1024 bytes against pathological adversarial input.
- **Leading-byte guard** — input not starting with `0x00` Object tag returns `Err` (W-2 acceptance). Unknown type tags log `tracing::warn!` + abort the branch with `Err`.
- **4 binary fixtures committed** from dev machine `C:\Program Files (x86)\Steam\appcache\stats`:
  - `UserGameStats_132274694_480.bin` (94 B, Spacewar test app — has `crc`, `PendingChanges`, 4 stat_slots)
  - `UserGameStatsSchema_480.bin` (1790 B)
  - `UserGameStats_132274694_2670630.bin` (85 B — has stat_slot 1 with `data=3` + AchievementTimes Object: bits 0+1 unlocked)
  - `UserGameStatsSchema_2670630.bin` (4537 B)
- **5 unit tests pass:** `parse_real_user_game_stats_fixture` (round-trip both real fixtures and asserts root_key non-empty + root is Object + contains `crc` / numeric stat_slot / `PendingChanges`); `unknown_type_tag_returns_err`; `non_zero_leading_byte_returns_err`; `recursion_depth_bounded`; `cstr_overlong_returns_err`.

### Task 2 — SteamLegitAdapter (commit `9cf48bb`)

- **Replaced steam_legit.rs stub** with the full `SourceAdapter` implementation. Shape mirrors `GoldbergAdapter` exactly:
  - `cached_watch_paths: Vec<PathBuf>` resolved at construction (WR-08 / BL-03 — no re-stat on hot dispatch path).
  - `baseline: Arc<RwLock<HashMap<(u64, String), bool>>>` keyed on `(app_id, ach_api_name)`.
  - `last_hash: Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>` for SHA-256 content-equality short-circuit.
  - **NEW vs Goldberg:** `schema_cache: Arc<RwLock<HashMap<u64, AppSchema>>>` mtime-keyed.
  - `on_file_changed` order: filename guard → user_id filter (Pitfall #5) → app_id parse → read with retry → SHA-256 hash compare → parse → diff against baseline → emit `RawUnlockEvent` per `false→true` transition → update baseline → update hash. Identical to Goldberg's commit-on-success ordering (BL-02).
- **`discover_paths`** reads `HKCU\Software\Valve\Steam\Users` enum_keys for user_ids. Fallback: scans `appcache/stats` filenames to extract a user_id (covers Steam installs without registry user history). Returns `appcache_stats: Option<PathBuf>` filtered by `path.exists()`. Non-Windows targets return default-empty.
- **`extract_state_mapping`** handles two real Steam encodings:
  - Plain stat-as-bool: `data: Int32 == 1` → emits `(stat_slot, 0) → true`.
  - Bit-mapped (real games): `AchievementTimes` Object presence with numeric bit-slot keys → emits `(stat_slot, bit_slot) → true` for each.
- **`extract_schema_mapping`** does deterministic path-walk (NOT a heuristic on `root.len() == 1`):
  1. Find any root child whose key parses as a numeric appid → descend into it.
  2. If none, treat root itself as the appid container.
  3. Locate `stats` Object (on container or root fallback) — log warn + return empty if absent.
  4. Walk numeric stat_slots reading direct `name` (bit_slot=0 plain) and per-`bits` sub-Object entries (per-bit names).
- **Missing-schema fallback (Pitfall #8):** `load_schema` returns empty map when the schema file is absent or fails to parse; `on_file_changed` then synthesises `ach_api_name = "steam_stat_<stat>_<bit>"`. Verified by `missing_schema_emits_placeholder_api_name` test.
- **Multi-user filter (Pitfall #5):** Both `seed_baseline` and `on_file_changed` drop entries whose filename-extracted user_id is not in `self.user_ids` (when that vec is non-empty). Logs at `tracing::debug` to keep production logs clean.
- **`read_bytes_with_retry`** binary equivalent of `goldberg::read_with_retry` — same 3-attempt × 50 ms × tokio::sleep loop on `ErrorKind::PermissionDenied` or raw_os_error 32 (ERROR_SHARING_VIOLATION) / 33 (ERROR_LOCK_VIOLATION).
- **8 unit tests pass:** filename parsing; real-fixture seed (no panic, baseline populated); unknown-user-id event drop; schema-filename event skip; SHA-256 identical-content short-circuit; synthetic state transition emit (verifies the false→true bit transition path); missing-schema placeholder name; `discover_paths(None)` returns default appcache.

## Pattern Alignment with GoldbergAdapter

The plan mandated mirroring Goldberg's exact shape. Where SteamLegit DIVERGES (intentionally):

| Aspect | Goldberg | SteamLegit | Why |
|---|---|---|---|
| File format | JSON (text) | Binary VDF | Steam writes binary KV, not JSON |
| State+schema | Single file (state only) | TWO files (state + schema) | Steam separates state from API-name mapping |
| Schema cache | N/A | `Arc<RwLock<HashMap<u64, AppSchema>>>` mtime-keyed | Schema files are large (1.5 KB – 1.3 MB); re-parsing on every event would be wasteful |
| File pattern | `<root>/<appid>/achievements.json` (depth 2) | `<appcache_stats>/UserGameStats_<userid>_<appid>.bin` (depth 1, flat) | Steam's appcache layout is flat — no `walkdir`, just `read_dir` |
| App ID resolution | parent dir parse + redirect_map fallback | filename regex split (`UserGameStats_<userid>_<appid>.bin`) | No directory hierarchy on Steam side |
| Multi-user filter | N/A | `user_ids: Vec<u64>` from registry | Steam supports multi-user Windows profiles; Goldberg does not |
| Missing-resource fallback | N/A | placeholder `steam_stat_<s>_<b>` ach_api_name | Schema can be absent for newly-installed games before first launch; popup still fires |
| Read function | `read_with_retry` returns `String` | `read_bytes_with_retry` returns `Vec<u8>` | Binary file vs text |
| `on_file_changed` order | filename guard → app_id → retry-read → hash → parse → diff → emit → update baseline → update hash | filename guard (state vs schema) → user_id filter → app_id → retry-read → hash → parse → diff → emit → update baseline → update hash | Identical commit-on-success ordering; SteamLegit adds two extra pre-filters (schema-filename skip + user_id filter) BEFORE I/O |

The Goldberg `read → hash → parse → diff → emit → THEN update baseline + hash` invariant (BL-02) is preserved verbatim.

## Deviations from Plan

None — plan executed exactly as written. The plan's code listings were copied to disk verbatim, all acceptance criteria checked green, and no auto-fix rules (1/2/3) triggered.

The plan's W-2 acceptance assertion (`has_known_key`) anticipated that real fixtures would contain `crc` / numeric stat_slot / `PendingChanges` — confirmed empirically in both committed fixtures (480 has all three; 2670630 has `crc` + `PendingChanges` + numeric "1" stat_slot).

## Threat-Model Coverage

The plan's threat register listed 6 threats with `mitigate` disposition. All were implemented:

| Threat ID | Mitigation Implemented |
|---|---|
| T-31-T1 (Tampering UserGameStats) | `parse_binary_vdf` returns `anyhow::Result`; depth bounded at 16; cstr bounded at 1024 B; unknown tags abort branch; `on_file_changed` logs warn + returns Ok on any parse failure (does not panic, does not poison baseline). |
| T-31-T2 (Tampering Schema) | Same defensive parsing. Missing schema → placeholder `steam_stat_*` ach_api_name (popup still fires). |
| T-31-D1 (DoS appcache/stats) | `seed_baseline` does flat `read_dir` (no recursive walkdir); per-file processing bounded by parser depth + cstr length. 166 dev-machine files seed in <100 ms (estimated from Goldberg's 50-file timing). |
| T-31-D2 (DoS deep VDF) | `MAX_RECURSION_DEPTH = 16` constant; verified by `recursion_depth_bounded` test failing on 20-deep input. |
| T-31-S1 (Spoofing user_id) | Filename pattern parsed via `parse_user_id_from_filename`; events whose user_id is not in `self.user_ids` (registry-discovered) drop at debug level. Verified by `on_file_changed_drops_unknown_user_id` test. |
| T-31-I1 (Info disclosure tracing) | `accept` disposition — local stdout only; same posture as Phase 1. No code changes needed. |

## Authentication Gates

None — this plan is purely local-file/registry I/O.

## Issues Encountered

- PowerShell-via-Bash variable interpolation collisions: bash kept stripping `$src=` style assignments. Worked around by writing PS scripts to `%TEMP%` and invoking via `powershell -File`. Same pattern as Plan 03-00's `hex_dump.ps1`.

## Plan 03-04 Readiness

The SteamLegitAdapter is fully ready for `lib.rs::run()` wiring (Plan 03-04). The construction signature is:
```rust
SteamLegitAdapter::new(
    discovered.steam_legit_appcache_stats,   // Option<PathBuf>
    discovered.steam_legit_user_ids,          // Vec<u64>
)
```
Both fields are already populated by `paths::discover()` in Plan 03-00. The adapter implements the full `SourceAdapter` trait (`name`, `kind`, `watch_paths`, `seed_baseline`, `on_file_changed`), so `WatcherCore` can register it identically to `GoldbergAdapter` — no special-casing in the watcher dispatch.

## Self-Check: PASSED

Verified each created/modified file exists:
- `src-tauri/src/sources/vdf_binary.rs` — FOUND (254 lines; stub marker absent; MAX_RECURSION_DEPTH present; 9 hex tag arms; 5 tests)
- `src-tauri/src/sources/steam_legit.rs` — FOUND (683 lines; stub marker absent; impl SourceAdapter for SteamLegitAdapter present; load_schema mtime cache present; Pitfall #5 + #6 + #8 references; 8 tests)
- `src-tauri/tests/fixtures/steam_legit/UserGameStats_132274694_480.bin` — FOUND (94 B)
- `src-tauri/tests/fixtures/steam_legit/UserGameStatsSchema_480.bin` — FOUND (1790 B)
- `src-tauri/tests/fixtures/steam_legit/UserGameStats_132274694_2670630.bin` — FOUND (85 B)
- `src-tauri/tests/fixtures/steam_legit/UserGameStatsSchema_2670630.bin` — FOUND (4537 B)

Verified each commit exists:
- `23e125c` — FOUND (Task 1: feat(03-01) implement binary VDF reader with real fixture round-trip)
- `9cf48bb` — FOUND (Task 2: feat(03-01) implement SteamLegitAdapter with mtime-cached schema lookup)

Verified `cargo check --all-targets` exits 0 (clean compile, no warnings) and `cargo test --lib` reports `116 passed; 0 failed; 0 ignored`.

---
*Phase: 03-remaining-source-adapters*
*Completed: 2026-05-09*
