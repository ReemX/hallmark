---
phase: 01-detection-pipeline-foundation
plan: 03
subsystem: path-discovery
tags: [windows-registry, vdf, libraryfolders, goldberg, local-save, appmanifest, walkdir, tracing, A4]
requires:
  - "Plan 01-01 scaffold (winreg, keyvalues-parser, walkdir, dirs, tracing-subscriber pinned in src-tauri/Cargo.toml)"
  - "Plan 01-01 paths.rs stub already declared in lib.rs"
provides:
  - "DiscoveredPaths struct (steam_install, steam_libraries, goldberg_save_roots, goldberg_local_save_redirects)"
  - "GoldbergRedirect struct (target_path + app_id) — Plan 04 GoldbergAdapter input"
  - "discover() entry point that runs registry → VDF → walkdir at startup, logs every path"
  - "goldberg_watch_paths(&d) helper — union of default roots + redirect targets"
  - "goldberg_redirect_map(&d) helper — HashMap<PathBuf,u64> for adapter directory-name fallback"
  - "appmanifest_lookup(&library) pub(crate) helper — installdir → appid map"
  - "16 passing unit tests including a tracing-capture test that asserts Success Criterion #5"
affects:
  - "Plan 01-04 (GoldbergAdapter) — consumes DiscoveredPaths, calls goldberg_watch_paths and goldberg_redirect_map"
  - "Phase 3 plans — will consume steam_libraries to find userdata/<steamid>/<appid>/ paths"
  - "Phase 4 first-run wizard — will surface DiscoveredPaths to UI"
tech-stack:
  added:
    - "winreg 0.56 active use (HKLM + HKCU registry probing)"
    - "keyvalues-parser 0.2 active use (libraryfolders.vdf + appmanifest_*.acf)"
    - "walkdir 2.5 active use (steamapps/common DLL scan, max_depth(8))"
    - "dirs 6.0 active use (data_dir for %APPDATA%)"
    - "tracing-subscriber Layer impl for tracing-capture testing (no new dep)"
  patterns:
    - "Pure-ish discover() — single function call, side effects limited to one log call per discovered path"
    - "Two-VDF-location fallback (config\\ post-2022 master, steamapps\\ legacy) with case-insensitive top-key match"
    - "Numeric-string-key filter to discriminate library entries from metadata keys (TimeNextStatsReport, ContentStatsID)"
    - "Windows-only cfg gating with Linux stub returning None — keeps CI compilable"
    - "tracing-capture test pattern via custom Layer + scoped subscriber for log assertion"
    - "Defensive parser posture: parse failure → warn + return empty, never panic (T-03-T2/T3/T4)"
key-files:
  created:
    - ".planning/phases/01-detection-pipeline-foundation/01-03-SUMMARY.md"
  modified:
    - "src-tauri/src/paths.rs (was 2-line stub, now 917-line full implementation)"
key-decisions:
  - "GoldbergRedirect pairs target_path with app_id at discovery time (rather than re-resolving in adapter) so Plan 04 can identify games whose redirect target's parent directory is non-numeric (e.g. 'Save', 'data')"
  - "Two-VDF-location iteration: try config\\libraryfolders.vdf first (post-2022 master), fall back to steamapps\\libraryfolders.vdf (legacy/replicated) — first hit wins"
  - "Steam install root is implicitly prepended as a library if libraryfolders.vdf does not list it explicitly (it always implicitly is one)"
  - "appmanifest_lookup returns HashMap<String,u64> keyed on installdir (not appid) because the redirect-resolution flow walks DLL path → installdir, not appid → installdir"
  - "max_depth(8) on the walkdir scan — generous bound that comfortably covers all real installs (typical depth 2-4) without DoS risk on adversarial trees (T-03-D1 mitigation)"
  - "Tracing-capture test pattern uses tracing_subscriber::registry().with(layer) + tracing::subscriber::set_default for scoped capture — no global side effects on parallel tests"
  - "Trim path before resolution (raw.trim()) handles trailing CRLF/whitespace from text editors that auto-add it"
patterns-established:
  - "Path-discovery functions: one tracing::info! per discovered path category at success, tracing::warn! for missing/unparseable inputs, never panic"
  - "VDF parsing: case-insensitive top-key match, defensive parse failure handling, no string concatenation"
  - "Test fixture builders (fresh_tmp + write_appmanifest helpers) for hermetic on-disk tests"
requirements-completed: [DETECT-08]

# Metrics
duration: 3min
completed: 2026-05-08
---

# Phase 01 Plan 03: Path Discovery Summary

**`paths::discover()` reads HKLM+HKCU Steam install, parses both libraryfolders.vdf locations (post-2022 nested + legacy flat), enumerates 3 Goldberg default save roots, and resolves every `local_save.txt` redirect into a `GoldbergRedirect { target_path, app_id }` via appmanifest-driven appid lookup — all logged via tracing::info! at startup (Success Criterion #5).**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-05-07T22:33:13Z
- **Completed:** 2026-05-07T22:37:12Z
- **Tasks:** 2
- **Files modified:** 1 (`src-tauri/src/paths.rs`, 2 lines → 917 lines)

## Accomplishments

- **Steam half (Task 1).** Registry probe (HKLM `SOFTWARE\WOW6432Node\Valve\Steam\InstallPath` first, HKCU `Software\Valve\Steam\SteamPath` fallback) plus a `keyvalues-parser`-driven VDF parser that handles both the post-2022 nested format AND the legacy flat format. Numeric-string-key filtering correctly drops `TimeNextStatsReport` / `ContentStatsID` metadata keys from legacy VDFs.
- **Goldberg half (Task 2).** `goldberg_default_roots()` checks all three documented paths (`%APPDATA%\Goldberg SteamEmu Saves`, `%APPDATA%\GSE Saves`, `%PUBLIC%\Documents\Goldberg SteamEmu Saves`) filtered for existence. `scan_local_save_redirects()` walks each Steam library's `steamapps\common\` (max_depth 8) for `steam_api*.dll`, reads the sibling `local_save.txt`, resolves the path (absolute pass-through OR joined to DLL dir; trimmed for whitespace), validates existence, AND pairs the resolved redirect with the appid from the matching `appmanifest_*.acf` (matched by `installdir`).
- **appmanifest lookup helper.** `pub(crate) fn appmanifest_lookup(library) -> HashMap<String, u64>` parses every `appmanifest_*.acf` in `<library>\steamapps\` (top-level `AppState`, fields `appid` + `installdir`) and returns the lookup map. Stand-alone unit-tested.
- **Plan-04 contract surfaces.** `pub fn goldberg_watch_paths(&DiscoveredPaths) -> Vec<PathBuf>` returns the union of default save roots and redirect targets. `pub fn goldberg_redirect_map(&DiscoveredPaths) -> HashMap<PathBuf, u64>` returns the redirect-target → appid map Plan 04's `GoldbergAdapter::new(roots, redirect_map)` will consume to identify games whose redirect target lives under a non-numeric directory (e.g. `D:\Game1\Save\achievements.json`).
- **16 passing unit tests.** Five `tests_steam` cover both VDF formats, escape handling, empty-input safety, and a temp-dir on-disk integration. Eleven `tests_goldberg` cover default-roots existence filtering, appmanifest lookup, all five `local_save.txt` edge cases (absolute / relative / missing-target / no-file / trailing-whitespace / no-matching-appmanifest), the two helper accessors (`goldberg_watch_paths`, `goldberg_redirect_map`), and a tracing-capture test that asserts `log_discovery` emits at least one INFO event per discovery category — automated proof of Success Criterion #5.

## Task Commits

Each task was committed atomically:

1. **Task 1: Steam registry + libraryfolders.vdf parser** — `4554ae4` (feat)
2. **Task 2: Goldberg roots + local_save.txt resolver + appmanifest lookup** — `96949b0` (feat)

**Plan metadata:** (final commit) — pending after this SUMMARY is written.

## Files Created/Modified

- `src-tauri/src/paths.rs` — Was a 2-line doc-only stub from Plan 01-01; now a 917-line full implementation with `DiscoveredPaths` + `GoldbergRedirect` structs, `discover()` entry, `goldberg_watch_paths` + `goldberg_redirect_map` accessors, `parse_libraryfolders` + `parse_libraryfolders_text`, `goldberg_default_roots`, `appmanifest_lookup`, `extract_installdir_from_dll_path`, `scan_local_save_redirects`, `log_discovery`, plus `tests_steam` (5 tests) and `tests_goldberg` (11 tests).
- `.planning/phases/01-detection-pipeline-foundation/01-03-SUMMARY.md` — This file.

## Public-API Surface (consumed by Plan 04)

| Function | Signature | Purpose |
|----------|-----------|---------|
| `discover` | `pub fn discover() -> DiscoveredPaths` | Top-level entry point; runs registry + VDF + walkdir at startup; logs every discovered path |
| `goldberg_watch_paths` | `pub fn goldberg_watch_paths(&DiscoveredPaths) -> Vec<PathBuf>` | Union of `goldberg_save_roots` and redirect target paths — direct input to `GoldbergAdapter::new(roots, ...)` |
| `goldberg_redirect_map` | `pub fn goldberg_redirect_map(&DiscoveredPaths) -> HashMap<PathBuf, u64>` | Redirect-target-path → appid lookup for adapter directory-name fallback |
| `DiscoveredPaths` | struct | Public; all four fields are `pub` for direct field access where needed |
| `GoldbergRedirect` | struct | Public; `target_path: PathBuf, app_id: u64` |
| `appmanifest_lookup` | `pub(crate) fn appmanifest_lookup(&Path) -> HashMap<String, u64>` | Crate-internal helper; reusable by future plans that need the same installdir→appid map |

## Resolution Algorithms

### Steam install registry probe order (REQ DETECT-08)
1. `HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Valve\Steam` → value `InstallPath` (64-bit machine-wide)
2. Fallback: `HKEY_CURRENT_USER\Software\Valve\Steam` → value `SteamPath` (current-user)
3. Each result is validated with `path.exists()`; missing-on-disk is logged as warn and treated as absent (T-03-T1 mitigation).

### libraryfolders.vdf candidates (REQ DETECT-08)
1. `<SteamInstall>\config\libraryfolders.vdf` (post-2022 master)
2. `<SteamInstall>\steamapps\libraryfolders.vdf` (legacy / replicated)
3. First-found wins; both are case-insensitively keyed on the top-level `libraryfolders` / `LibraryFolders` token.
4. Steam install root is implicitly prepended to the result if not listed.

### Goldberg default save roots (PITFALLS.md #6)
1. `%APPDATA%\Goldberg SteamEmu Saves\` (legacy default)
2. `%APPDATA%\GSE Saves\` (gbe_fork default; majority of 2024+ scene releases)
3. `%PUBLIC%\Documents\Goldberg SteamEmu Saves\` (rare; older releases)
4. Each is filtered for `path.exists()` — function may return 0 to 3 entries.

### local_save.txt resolution (PITFALLS.md #6 + RESEARCH.md "Pitfall #6")
1. For each Steam library, walkdir `<library>\steamapps\common\` with `max_depth(8)`.
2. For each `steam_api.dll` or `steam_api64.dll` (case-insensitive), check sibling `local_save.txt`.
3. Read content, `trim()` whitespace.
4. If `Path::is_absolute()`, use as-is. Otherwise, join to DLL parent directory.
5. Validate resolved target with `path.exists()`; missing → warn + skip.
6. Walk DLL path back to find `<library>\steamapps\common\<installdir>` segment.
7. Look up `installdir` in `appmanifest_lookup(library)`; missing → warn + skip.
8. Pair resolved target with the appid into `GoldbergRedirect`.

## Decisions Made

| Decision | Rationale | Alternatives Considered |
|----------|-----------|-------------------------|
| Pair every `GoldbergRedirect` with its appid AT discovery time, not at adapter event-handling time | Plan 04's `GoldbergAdapter` will receive directory paths from notify events; if the redirect target's directory name is "Save" or "data" (not numeric), the adapter cannot recover the appid from the path alone. Pre-pairing during discovery removes that ambiguity. | Defer appid resolution to event-handling — discarded; that would force the adapter to also depend on `appmanifest_lookup`, which is a discovery concern, not an adapter concern. |
| `extract_installdir_from_dll_path` walks segment-by-segment looking for `steamapps`+`common` (case-insensitive) | Path comparisons on Windows must be case-insensitive (`STEAMAPPS\Common\Foo` is the same dir as `steamapps\common\Foo`). Using `OsStr::eq_ignore_ascii_case` is the lightest possible normalization. | Lowercasing the entire path with `to_string_lossy().to_lowercase()` — discarded; loses round-trip fidelity for the returned `installdir` string. |
| Tracing-capture test pattern uses `tracing::subscriber::set_default` (scoped) not `set_global_default` | Tests run in parallel by default. A global subscriber would leak across tests and cause flakes. `set_default` returns a guard that limits capture to the current thread/scope. | Sequential test marker — discarded; would slow the suite. |
| `max_depth(8)` on the walkdir scan | Generous bound covering all real game installs (typical depth 2-4 for `<game>\bin\steam_api64.dll`). Bounds adversarial-tree DoS (T-03-D1 mitigation). | Unbounded walkdir — discarded; a maliciously-deep symlink loop could DoS startup. |
| `pub(crate)` visibility on `appmanifest_lookup` (not `pub`) | The function is used by `scan_local_save_redirects` and may be reused by future Phase 3 plans within the same crate, but external crates have no business reading appmanifests directly. `pub(crate)` is the minimum visibility that satisfies both. | `pub` (over-exposure) or private (forces a re-implementation in Phase 3 if needed). |

## Deviations from Plan

None — plan executed exactly as written.

The plan's reference implementation in the `<action>` block compiled and tested clean on the first attempt. `cargo fmt` made one cosmetic line-wrap in the tracing-capture test (folding a single-line `.iter().any(...)` chain onto three lines for the 100-column limit); this is the standard fmt behavior and was applied before commit, so no separate fix-up commit was needed.

## Issues Encountered

None.

## Authentication Gates

None occurred during this plan.

## Threat Surface Compliance

The plan's `<threat_model>` lists six threats (T-03-T1 through T-03-D1, T-03-I1). Implementation status:

| Threat | Disposition | Mitigation status |
|--------|-------------|-------------------|
| T-03-T1 (registry InstallPath tampering) | mitigate | `path.exists()` validation after registry read; warn + return None on mismatch |
| T-03-T2 (libraryfolders.vdf tampering) | mitigate | Defensive `keyvalues-parser` parse, warn + empty Vec on parse failure, no panic |
| T-03-T3 (local_save.txt tampering / path traversal) | mitigate | Resolved path is only used as a watch target — never written to or executed; existence-validated before use |
| T-03-T4 (appmanifest tampering) | mitigate | Same defensive parser; non-numeric `appid` strings are silently skipped; `installdir` strings are only used as HashMap keys |
| T-03-D1 (walkdir DoS) | mitigate | `max_depth(8)` on the scan; appmanifest_lookup is non-recursive |
| T-03-I1 (tracing logs include user paths) | accept | Local-only stdout logging; no telemetry |

## Verification Output

```
$ cargo check --manifest-path src-tauri/Cargo.toml --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.82s

$ cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
(no output — clean)

$ cargo test --manifest-path src-tauri/Cargo.toml --lib paths
running 16 tests
test paths::tests_goldberg::goldberg_redirect_map_keys_on_target_path ... ok
test paths::tests_goldberg::goldberg_watch_paths_combines_roots_and_redirects ... ok
test paths::tests_steam::parse_libraryfolders_empty_text_returns_empty ... ok
test paths::tests_steam::parse_libraryfolders_handles_escapes ... ok
test paths::tests_steam::parse_libraryfolders_legacy_flat ... ok
test paths::tests_steam::parse_libraryfolders_post_2022_nested ... ok
test paths::tests_goldberg::tracing_capture_records_info_event_for_each_discovery_category ... ok
test paths::tests_goldberg::goldberg_default_roots_returns_only_existing ... ok
test paths::tests_steam::parse_libraryfolders_wraps_text_in_outer_disk_paths ... ok
test paths::tests_goldberg::local_save_no_local_save_txt_skipped ... ok
test paths::tests_goldberg::appmanifest_lookup_returns_appid_for_installdir ... ok
test paths::tests_goldberg::local_save_missing_target_is_filtered_out ... ok
test paths::tests_goldberg::local_save_relative_path_resolves_against_dll_dir ... ok
test paths::tests_goldberg::local_save_absolute_path_passes_through ... ok
test paths::tests_goldberg::local_save_trims_trailing_whitespace ... ok
test paths::tests_goldberg::local_save_no_matching_appmanifest_is_skipped ... ok
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out
```

## Next Plan Readiness

Plan 04 (`01-04-goldberg-adapter-and-watcher-core`) can now:
- `use hallmark_lib::paths::{discover, goldberg_watch_paths, goldberg_redirect_map, DiscoveredPaths, GoldbergRedirect};`
- Call `paths::discover()` once at startup in `lib.rs::run()`'s `setup()` closure.
- Pass `goldberg_watch_paths(&d)` as the `roots` argument to `GoldbergAdapter::new(...)`.
- Pass `goldberg_redirect_map(&d)` as the `redirect_map` argument so the adapter can resolve a notify event's parent directory to its appid even when the parent's name is not numeric.

REQ DETECT-08 is fully covered. Success Criterion #5 ("all discovered paths logged at startup") is now covered by an automated test (`tracing_capture_records_info_event_for_each_discovery_category`) that asserts `log_discovery` emits at least one INFO event per category.

## Self-Check: PASSED

- `src-tauri/src/paths.rs` exists (917 lines).
- Contains `pub struct DiscoveredPaths`, `pub struct GoldbergRedirect`, `pub fn discover()`, `pub fn goldberg_watch_paths`, `pub fn goldberg_redirect_map`, `pub(crate) fn parse_libraryfolders_text`, `pub(crate) fn appmanifest_lookup`, `fn goldberg_default_roots`, `fn scan_local_save_redirects`, `fn extract_installdir_from_dll_path`, `fn log_discovery`.
- Contains literal `WOW6432Node\Valve\Steam` and `Software\Valve\Steam` registry paths.
- Contains `keyvalues_parser::Vdf` and `walkdir::WalkDir` API usage.
- Contains all three Goldberg default-root strings: `Goldberg SteamEmu Saves`, `GSE Saves`, `PUBLIC`.
- Contains `is_absolute()`, `AppState`, `installdir` literal references.
- Contains `tracing_subscriber::registry()` for the capture test.
- Commits exist on master: `4554ae4` (Task 1), `96949b0` (Task 2) — verified via `git log --oneline`.
- `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` returns exit 0.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` returns exit 0 (clean).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib paths` runs 16 tests and all pass.
