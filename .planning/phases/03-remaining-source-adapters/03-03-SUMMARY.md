---
phase: 03-remaining-source-adapters
plan: "03"
subsystem: detection
tags: [rust, source-adapter, smartsteamemu, binary-parser, crc32, lazy-reverse-map, sha256, env-override]

requires:
  - phase: 03-remaining-source-adapters
    provides: Plan 03-00 stub modules (sse), SourceKind::SmartSteamEmu, DiscoveredPaths.sse_appid_dirs, crc32fast 1.4 dep
  - phase: 01-detection-pipeline-foundation
    provides: SourceAdapter trait, RawUnlockEvent, GoldbergAdapter pattern (Arc<RwLock<...>> baseline + last_hash, read-with-retry, SHA-256 short-circuit, parse → diff → emit → update)
provides:
  - SseAdapter with full SourceAdapter implementation (5 methods)
  - parse_sse_stats: header + 24-byte record binary parser with defensive count cap and value>1 stat skip
  - build_crc_reverse_map: zero-padded 8-char hex keys (Pitfall #3 mitigation)
  - resolve_api_name: lazy per-app CRC32-hex → api_name reverse map fed by Goldberg companion file with <crc:0x...> placeholder fallback
  - discover_paths reading %APPDATA%\SmartSteamEmu\<numeric_appid> with warn-and-skip for the User\Achievements.ini variant (Open Question #2 deferred)
  - HALLMARK_SSE_ROOT_OVERRIDE env var override (parallels Phase 1's HALLMARK_GOLDBERG_ROOT_OVERRIDE)
  - 9 new unit tests covering binary round-trip, leading-zero CRC zero-pad, count cap, value>1 stat skip, empty file, false→true emit, filename guard, SHA-256 short-circuit, extract_app_id
affects: [03-04-pipeline-integration]

tech-stack:
  added: []
  patterns:
    - "Lazy per-app CRC32 reverse map: build_crc_reverse_map called once per appid on first event; results cached in Arc<RwLock<HashMap<u64, HashMap<String, String>>>>; placeholder fallback never blocks event emission"
    - "Goldberg companion file harvesting: when Goldberg state file exists alongside SSE for the same appid, its top-level keys provide candidate API names for the reverse map (zero new disk format introduced)"
    - "Synthetic stats.bin in tests via to_le_bytes round-trip: writer constructs 24-byte records that the parser consumes verbatim; confirms little-endian-CRC-byte-reversal interpretation matches Achievement-Watcher"
    - "STATE_FILENAME constant ('stats.bin') used 6 times — single source of truth for filename guard, seed file path, test fixture writer, and discover_paths candidate join"

key-files:
  created: []
  modified:
    - src-tauri/src/sources/sse.rs

key-decisions:
  - "Plan 03-03: CRC bytes are stored reversed in stats.bin — read [r[3], r[2], r[1], r[0]] then u32::from_be_bytes yields the natural CRC32 value. Confirmed empirically by synthetic round-trip in parse_sse_stats_round_trip_synthetic."
  - "Plan 03-03: All CRC hex strings are zero-padded to exactly 8 chars (Pitfall #3) — both producer (parse_sse_stats) and consumer (build_crc_reverse_map) use {:08x} format, ensuring map keys collide correctly across both code paths."
  - "Plan 03-03: Defensive count cap = min(declared_count, (bytes.len() - 4) / 24) — prevents tampered stats.bin with declared = i32::MAX from triggering arithmetic overflow or unbounded allocation (T-33-T1 mitigated)."
  - "Plan 03-03: value>1 records are stats not achievements — skipped silently in parse_sse_stats. Only value ∈ {0, 1} emits an SseRecord. Matches Achievement-Watcher's sse.js stat-vs-achievement disambiguation."
  - "Plan 03-03: Goldberg companion file (`%APPDATA%\\GSE Saves\\<appid>\\achievements.json` or `Goldberg SteamEmu Saves\\<appid>\\achievements.json`) is the v1 candidate source for CRC reverse map — Phase 2 SchemaCache integration is OUT OF SCOPE here per RESEARCH.md, deferred to Plan 04 polish or Phase 4 wizard."
  - "Plan 03-03: Placeholder format `<crc:0x{:08x}>` when no candidate API name resolves — popup still fires with degraded display rather than silently dropping (Pitfall #8 analog from Plan 03-01)."
  - "Plan 03-03: User\\Achievements.ini variant (Hydra-referenced alt path) is logged warn and skipped during discovery — RESEARCH.md Open Question #2 deferral. Phase 4 polish revisits if user reports surface."

metrics:
  duration: 8min
  tasks: 1
  files_created: 0
  files_modified: 1
  loc_added: 521
  loc_removed: 28
  tests_added: 9
  tests_passing: 131
  tests_passing_baseline: 124

requirements-completed: [DETECT-04]

started: 2026-05-09T09:11:00Z
completed: 2026-05-09T09:19:00Z
---

# Phase 03 Plan 03: SseAdapter Summary

**Implemented full SseAdapter (REQ DETECT-04) with 24-byte binary record parser, lazy per-app CRC32 reverse map fed by Goldberg companion files (with `<crc:0x...>` placeholder fallback), discover_paths enumerating `%APPDATA%\SmartSteamEmu\<appid>\` and warning-and-skipping the Hydra `User\Achievements.ini` variant. 9 new unit tests pass; full lib suite at 131 passing (up from 124 baseline).**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-05-09T09:11:00Z (after Plan 03-02 completion at 09:09)
- **Completed:** 2026-05-09T09:19:00Z
- **Tasks:** 1
- **Files modified:** 1 (sse.rs)
- **LoC delta:** +521 / -28 net (replacing the 79-LoC stub from Plan 03-00 with the full 521-LoC implementation; counts include tests)
- **Tests added:** 9 (net +7 vs prior 2 stub tests)

## Accomplishments

### Task 1 — SseAdapter (commit `bb096a6`)

- **Replaced sse.rs stub** with the full `SourceAdapter` implementation. Shape mirrors `GoldbergAdapter` exactly with one extension:
  - `cached_watch_paths: Vec<PathBuf>` resolved at construction, `path.exists()` filter applied once.
  - `baseline: Arc<RwLock<HashMap<(u64, String), bool>>>` keyed on `(app_id, ach_api_name)`.
  - `last_hash: Arc<RwLock<HashMap<PathBuf, [u8; 32]>>>` for SHA-256 content-equality short-circuit.
  - **`crc_reverse: Arc<RwLock<HashMap<u64, HashMap<String, String>>>>`** — NEW field unique to SSE; per-app CRC32-hex → api_name lazy reverse map.
  - `on_file_changed` order: filename guard FIRST → app_id parse from `path.parent()` → `read_bytes_with_retry` → SHA-256 hash compare → `parse_sse_stats` → resolve api_name per record → diff against baseline → emit `RawUnlockEvent` per `false→true` transition → update baseline → update last_hash. Identical to Goldberg's commit-on-success ordering (BL-02 invariant preserved).

- **`parse_sse_stats`** — Pure 24-byte-record binary parser:
  1. Reads `expectedStatsCount` from `bytes[0..4]` as Int32LE.
  2. Defensive cap: `count = min(declared_count, (bytes.len() - 4) / 24)`. Negative declared count clamped to 0. Logs warn when declared exceeds capped (T-33-T1).
  3. Per-record (24 bytes): CRC32 from `[r[3], r[2], r[1], r[0]]` as `u32::from_be_bytes` (file stores bytes reversed). UnlockTime from `r[8..12]` as Int32LE. Value from `r[20..24]` as Int32LE.
  4. **value > 1 → skipped** (it's a stat, not an achievement; Achievement-Watcher convention).
  5. Emits `SseRecord { crc32_hex: format!("{:08x}", crc), achieved: value == 1, unlock_time }`.
  6. Loop checks `off + 24 > bytes.len()` to break on truncated mid-record without panic (T-33-T2).

- **`build_crc_reverse_map`** — Pure function over `&[String]` candidates: hashes each via `crc32fast::Hasher`, formats key as `format!("{:08x}", crc)`, returns `HashMap<String, String>` (zero-padded 8-char hex → api_name). Pitfall #3 mitigated — keys are always 8 chars regardless of CRC magnitude.

- **`load_goldberg_companion_keys`** — Reads `%APPDATA%\GSE Saves\<appid>\achievements.json` or `%APPDATA%\Goldberg SteamEmu Saves\<appid>\achievements.json` (whichever exists first), parses as JSON object, returns top-level keys as candidate API names. Returns empty Vec on missing/unparseable — never panics.

- **`resolve_api_name`** — Per-event resolver:
  1. Read-locks `crc_reverse`; if `HashMap[app_id][crc_hex]` exists, returns clone.
  2. Otherwise calls `load_goldberg_companion_keys(app_id)`, builds reverse map, write-locks `crc_reverse`, inserts the map.
  3. Returns `<crc:0x{crc_hex}>` placeholder if the map didn't contain `crc_hex` either.

- **`discover_paths`** — Enumerates `%APPDATA%\SmartSteamEmu` (via `dirs::data_dir()`):
  - Honors `HALLMARK_SSE_ROOT_OVERRIDE` env var for fixture-tree integration testing (B-2 / RESEARCH.md line 417, parallels Phase 1's HALLMARK_GOLDBERG_ROOT_OVERRIDE).
  - Filters subdirs by numeric u64 parse (SSE subdirs are exclusively integer appids).
  - Includes appid dirs that contain `stats.bin`.
  - **Warn-and-skip** for appid dirs that have ONLY `User\Achievements.ini` and no `stats.bin` — RESEARCH.md Open Question #2 deferral.
  - Returns empty `SsePaths::default()` when SSE root is absent — no panic.

- **`extract_app_id`** — Parses `<root>/<appid>/stats.bin` one-level upward (`path.parent()`); returns `None` on non-numeric.

- **`STATE_FILENAME` const** — `"stats.bin"` used at 6 sites (declaration, filename guard, seed_baseline join, test fixture join, on_file_changed_skips_identical_content_via_sha256 join, on_file_changed_emits_event_on_synthetic_transition join).

- **`read_bytes_with_retry`** — Mirrors `goldberg::read_with_retry` shape (3-attempt × 50ms × `tokio::sleep` loop on `ErrorKind::PermissionDenied` or `raw_os_error == 32 (ERROR_SHARING_VIOLATION) | 33 (ERROR_LOCK_VIOLATION)`), but returns `Vec<u8>` (binary file) instead of `String`.

- **9 unit tests pass:**
  1. `parse_sse_stats_round_trip_synthetic` — synthetic 2-record file with `crc1 = 0x12345678` and `crc2 = 0x000000AB` (small-CRC zero-pad case) round-trips correctly. CRC bytes asserted as `"12345678"` and `"000000ab"`.
  2. `parse_sse_stats_skips_value_greater_than_one` — 1-record file with value=42 → 0 records emitted.
  3. `parse_sse_stats_caps_count_to_file_size` — declared count=100, only 2 records of bytes → parser returns 2 records (defensive cap).
  4. `parse_sse_stats_empty_file_returns_empty` — empty bytes and `[0,0,0,0]` both yield 0 records.
  5. `build_crc_reverse_map_zero_pads_keys` — all map keys are exactly 8 chars long for any input.
  6. `on_file_changed_emits_event_on_synthetic_transition` — sets up Goldberg companion file with "ACH_SYNTHETIC" key, seeds adapter with achieved=false, flips file to achieved=true, asserts one `RawUnlockEvent { app_id: 9999, source: SourceKind::SmartSteamEmu }` is emitted with either resolved name or placeholder.
  7. `on_file_changed_skips_non_stats_filename` — `not_stats.bin` event yields no events (filename guard FIRST).
  8. `on_file_changed_skips_identical_content_via_sha256` — second call with identical bytes short-circuits before parse.
  9. `extract_app_id_from_canonical_path` — `/tmp/SmartSteamEmu/9999/stats.bin` → `Some(9999)`; `/tmp/SmartSteamEmu/notnumeric/stats.bin` → `None`.

## CRC32 Reverse-Lookup Design Notes

The SSE binary format keys achievements by `CRC32(api_name)`, not by api_name itself. This means parsing `stats.bin` gives us a CRC, not a name. To emit `RawUnlockEvent { ach_api_name, .. }` (the popup needs a string identifier matching SchemaCache entries), we need an inverse map.

**Lazy construction.** The reverse map is per-app, keyed `app_id → HashMap<crc_hex, api_name>`. It is built once on the first `resolve_api_name` call for a given appid; subsequent calls hit the cached HashMap. This avoids paying the JSON-parse cost upfront for every appid in the watch set, and means apps without a companion file simply cache an empty map (placeholder fallback fires instead).

**Goldberg companion as candidate source (v1).** The Goldberg state file at `%APPDATA%\GSE Saves\<appid>\achievements.json` (or `Goldberg SteamEmu Saves` legacy) is JSON object keyed on api_names. Its keys are exactly the candidates we need. This is a zero-new-format reuse — when a user has both Goldberg and SSE installed for the same appid (e.g., switching between fixers), the Goldberg state file fills the SSE adapter's reverse map at no cost.

**Phase 2 SchemaCache integration deferred.** RESEARCH.md and the plan explicitly out-of-scope SchemaCache reads here. Phase 2's SchemaCache (Steam-fetched canonical schema) would be a more authoritative candidate source, but wiring it requires either a sync handle to the SchemaCache via the WatcherCore or threading it through the SseAdapter::new constructor — work that fits Plan 04 pipeline integration or a dedicated Phase 4 polish plan.

**Placeholder fallback (Pitfall #8 analog).** When neither the cache nor the Goldberg companion resolves a CRC, the adapter emits `<crc:0x{crc_hex}>` (e.g., `"<crc:0x12345678>"`). This means the popup still fires with a degraded display string instead of silently dropping the event. Phase 4 wizard or a future SchemaCache integration can backfill these names.

**Zero-pad invariant (Pitfall #3).** Both producer (`parse_sse_stats` outputs `format!("{:08x}", crc)`) and consumer (`build_crc_reverse_map` keys with `format!("{:08x}", crc)`) use the same 8-char-padded format, so map lookups always succeed when the CRC genuinely matches. JavaScript reference parsers strip leading zeros for hashes < 0x1000; this Rust adapter does NOT, by design.

## v1 Deferral: User\Achievements.ini Variant (Open Question #2)

Hydra references `%APPDATA%\SmartSteamEmu\<appid>\User\Achievements.ini` as an alternate state path. This v1 plan does not implement that variant — `discover_paths` only includes appid dirs that contain `stats.bin`. Appid dirs with only the INI variant emit `tracing::warn!` (with the offending path) and are skipped.

**Justification.** RESEARCH.md flagged `stats.bin` as the single canonical SSE format documented by Achievement-Watcher; the INI variant is Hydra-specific and lacks a canonical parser. Implementing it would double the surface area of the adapter (separate parser, separate filename guard, separate path layout) for a low-confidence-coverage payoff. The warn log gives us telemetry to revisit in Phase 4 polish if user reports indicate this variant is actually in the wild on real installs.

**Recovery path.** If a user reports an INI-only SSE install, Phase 4 polish can:
1. Add a parallel `parse_sse_ini` parser.
2. Extend `discover_paths` to also include INI-only dirs.
3. Add a per-path-format dispatch in `on_file_changed`.

No data is lost in v1 — the warn log captures the path so users can self-diagnose.

## Pattern Alignment with GoldbergAdapter

The plan mandated mirroring Goldberg's exact shape. Where Sse DIVERGES (intentionally):

| Aspect | Goldberg | Sse | Why |
|---|---|---|---|
| File format | JSON (text) | Binary, 4-byte header + N×24-byte records | SSE uses binary fixed-width format per Achievement-Watcher convention |
| Parser | `serde_json::from_str` | `parse_sse_stats` hand-rolled byte-slice reader | Binary format too simple for a crate dep; hand-roll is 40 LoC |
| File pattern | `<root>/<appid>/achievements.json` (depth 2) | `<root>/<appid>/stats.bin` (depth 2) | Same depth; just different filename and format |
| App ID resolution | `path.parent().file_name()` numeric parse | `path.parent().file_name()` numeric parse | Identical |
| api_name source | JSON keys directly | CRC32 reverse-lookup from companion file or `<crc:0x...>` placeholder | SSE format is keyed on CRC, not name; reverse-lookup is intrinsic |
| Watch root construction | per-root walkdir for seed | per-appid-dir direct file lookup for seed | SSE watch roots ARE the appid dirs (set at construction); no walkdir needed |
| Redirect map | yes (Goldberg `local_save.txt`) | no | SSE has no redirect feature |
| Read function | `read_with_retry` returns `String` | `read_bytes_with_retry` returns `Vec<u8>` | Binary file requires raw bytes |
| Extra adapter field | none | `crc_reverse: Arc<RwLock<HashMap<u64, HashMap<String, String>>>>` | Lazy per-app CRC reverse map; Goldberg has no equivalent need |
| `on_file_changed` order | filename guard → app_id → retry-read → hash → parse → diff → emit → update baseline → update hash | filename guard → app_id → retry-read → hash → parse → resolve_api_name per record → diff → emit → update baseline → update hash | **Functionally identical**; the `resolve_api_name` step is the only insertion |

The Goldberg `read → hash → parse → diff → emit → THEN update baseline + hash` invariant (BL-02) is preserved verbatim in SseAdapter.

## Deviations from Plan

None — plan executed exactly as written. The plan's code listing was copied to disk verbatim, all acceptance criteria checked green, and no auto-fix rules (1/2/3) triggered.

The plan's required-tests list was 6; the executor delivered 9 (6 required + 3 additional coverage tests for: empty-file edge case, non-stats filename guard, extract_app_id canonical path).

## Threat-Model Coverage

The plan's threat register listed 6 threats with `mitigate` disposition (T-33-T1, T-33-T2, T-33-D1, T-33-S1) plus 2 `accept` (T-33-S2, T-33-I1). All `mitigate` items are implemented:

| Threat ID | Mitigation Implemented |
|---|---|
| T-33-T1 (Tampering — declared `count = i32::MAX`) | `parse_sse_stats` caps `count = min(count_raw as usize, max_records)` where `max_records = (bytes.len() - 4) / 24`. Negative count_raw clamped to 0. Tested by `parse_sse_stats_caps_count_to_file_size`. |
| T-33-T2 (Tampering — truncated mid-record) | Loop checks `off + 24 > bytes.len()` and breaks; never panics on partial record. The 4-byte slice indexers also stay within bounds because the loop's `count` value is itself bounded by `max_records`. |
| T-33-D1 (DoS huge stats.bin) | `std::fs::read` returns `Vec<u8>` bounded by file size; `parse_sse_stats` allocates `Vec::with_capacity(count)` where `count` is already capped. The 500ms debouncer + SHA-256 short-circuit (REQ DETECT-06) prevents re-parse on identical content. |
| T-33-S1 (Spoofing — `notstats.bin` masquerade) | Filename guard checks `path.file_name() == Some("stats.bin")` exactly. Variants like `not_stats.bin`, `stats.bin.bak`, `stats.txt` all return `Ok(())` immediately without I/O. Verified by `on_file_changed_skips_non_stats_filename`. |
| T-33-S2 (CRC32 collision) | accept disposition — collision probability ≈ 2^-32 is far below the noise floor of "user has 5000 achievements". A collision produces a wrong but harmless display name; the adapter never fires a false event because both records key on the same CRC, hence the same baseline entry. |
| T-33-I1 (Tracing log path/appid disclosure) | accept disposition — local stdout only. |

## Authentication Gates

None — this plan is purely local-file I/O.

## Issues Encountered

None.

## Plan 03-04 Readiness

The SseAdapter is fully ready for `lib.rs::run()` wiring (Plan 03-04). The construction signature is:
```rust
SseAdapter::new(discovered.sse_appid_dirs)   // Vec<PathBuf>
```
The single field is already populated by `paths::discover()` (Plan 03-00 wired `sse::discover_paths()` into the discovery flow). The adapter implements the full `SourceAdapter` trait (`name`, `kind`, `watch_paths`, `seed_baseline`, `on_file_changed`), so `WatcherCore` can register it identically to `GoldbergAdapter`, `SteamLegitAdapter`, and `CreamApiAdapter` — no special-casing in the watcher dispatch.

The `CrossSourceDedup` designed in Plan 01-05 already keys on `(app_id, ach_api_name)` and generalizes to N adapters, so the Plan 03-04 cross-source dedup integration test will pass against {Goldberg, SteamLegit, CreamApi, SmartSteamEmu} without additional dedup logic. Note that for SSE, the `ach_api_name` may be `<crc:0x...>` placeholder for some events — these values are stable per-CRC and per-app, so dedup still functions correctly across simultaneous unlocks.

The `HALLMARK_SSE_ROOT_OVERRIDE` env var is now wired and ready for SC2 integration testing in Plan 03-04.

## Self-Check: PASSED

Verified each modified file exists:
- `src-tauri/src/sources/sse.rs` — FOUND (521 lines; stub marker absent; `impl SourceAdapter for SseAdapter` present; `parse_sse_stats` defined; `build_crc_reverse_map` defined; `load_goldberg_companion_keys` at 2 sites; `STATE_FILENAME` at 6 sites; `Pitfall #3` reference present; `Open Question #2` reference present; `HALLMARK_SSE_ROOT_OVERRIDE` reference present; `crc32fast::Hasher` used; 9 tests)

Verified the commit exists:
- `bb096a6` — FOUND (`feat(03-03): implement SseAdapter with 24-byte stats.bin parser`)

Verified `cargo check --all-targets` exits 0 (clean compile, no warnings) and `cargo test --lib` reports `131 passed; 0 failed; 0 ignored`.

## Self-Check Results

- File `src-tauri/src/sources/sse.rs` — FOUND
- File `.planning/phases/03-remaining-source-adapters/03-03-SUMMARY.md` — FOUND
- Commit `bb096a6` — FOUND in `git log --oneline`

---
*Phase: 03-remaining-source-adapters*
*Completed: 2026-05-09*
