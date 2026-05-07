---
phase: 01-detection-pipeline-foundation
plan: 01
subsystem: build-foundation
tags: [scaffold, tauri, cargo, dependencies, goldberg-schema, A4]
requires:
  - "Tauri 2.x WebView2 runtime present on Windows"
  - "Rust toolchain stable (1.85+)"
  - "crates.io reachable for `cargo fetch`"
provides:
  - "Buildable Cargo workspace with `src-tauri/` member"
  - "All 18 Phase 1 dependencies pinned and resolved"
  - "Tauri runtime skeleton with empty `windows` array (backend-only)"
  - "tracing-subscriber wired (stdout, RUST_LOG-driven)"
  - "Module stubs for paths/sources/store/watcher (filled by Plans 02–05)"
  - "thiserror-based error enums (PathDiscoveryError, AdapterError, StoreError)"
  - "Empirically confirmed Goldberg/gbe_fork state-file schema (A4 resolution)"
  - "Canonical Spacewar (appid 480) test fixture for downstream parser tests"
affects:
  - "Plans 02–05 in Phase 1 (depend on this scaffold)"
  - "Phase 2 (frontend will populate `windows` and `dist/`)"
  - "Phase 4 (bundling will flip `bundle.active` and replace placeholder icon)"
tech-stack:
  added:
    - "tauri 2.11"
    - "tauri-build 2.6"
    - "tokio 1.52"
    - "notify 8.2"
    - "notify-debouncer-full 0.7"
    - "serde 1.0 + serde_json 1.0"
    - "rusqlite 0.39 (bundled)"
    - "keyvalues-parser 0.2"
    - "winreg 0.56 (Windows-only target dep)"
    - "walkdir 2.5"
    - "sha2 0.11"
    - "anyhow 1.0"
    - "thiserror 2.0"
    - "async-trait 0.1"
    - "tracing 0.1 + tracing-subscriber 0.3"
    - "dirs 6.0"
    - "uuid 1.23"
  patterns:
    - "Workspace + single binary crate, resolver=2"
    - "Library crate (`hallmark_lib`) + binary crate (`hallmark`) sharing src/"
    - "thiserror per-module error enums; anyhow at app boundary"
    - "tracing-subscriber with EnvFilter (RUST_LOG-overridable, info default)"
    - "Tauri `setup()` hook reserved as the single attachment point for downstream pipeline tasks"
key-files:
  created:
    - "Cargo.toml"
    - "Cargo.lock"
    - ".gitignore"
    - "src-tauri/Cargo.toml"
    - "src-tauri/build.rs"
    - "src-tauri/tauri.conf.json"
    - "src-tauri/icons/icon.ico"
    - "src-tauri/src/main.rs"
    - "src-tauri/src/lib.rs"
    - "src-tauri/src/error.rs"
    - "src-tauri/src/paths.rs"
    - "src-tauri/src/sources/mod.rs"
    - "src-tauri/src/watcher/mod.rs"
    - "src-tauri/src/store/mod.rs"
    - "dist/index.html"
    - "tests/fixtures/goldberg/480/achievements.json"
    - "tests/fixtures/goldberg/README.md"
    - ".planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md"
  modified: []
decisions:
  - "Pinned `tauri-build` to 2.6 (its actual latest stable on crates.io), not 2.11 — the build helper crate's version track is independent of the tauri runtime."
  - "Pinned `notify-debouncer-full` to 0.7 (corrects STACK.md's stale 0.5 citation)."
  - "Generated a multi-layer transparent placeholder ICO (16/24/32/48/64/256, 32bpp) so tauri-build's mandatory Windows resource step compiles. Real branding lives in Phase 4."
  - "Committed a placeholder `dist/index.html` because `tauri::generate_context!()` validates `frontendDist` exists at compile time, and Phase 1 has no Vite build. `.gitignore` whitelists only the placeholder."
  - "Set `bundle.targets = \"all\"` (with `bundle.active = false`) — the plan's `\"none\"` is not a valid bundle target enum value."
  - "Resolved Goldberg state-file schema (Assumption A4) by direct observation of three real gbe_fork saves on the developer machine — schema confirmed identical to legacy Goldberg per RESEARCH.md secondary sources."
metrics:
  duration_minutes: 10
  completed_date: "2026-05-08"
  tasks_completed: 3
  tasks_total: 3
  files_created: 18
  files_modified: 0
  commits: 3
---

# Phase 01 Plan 01: Tauri Rust Scaffold Summary

Bootstrapped a buildable Cargo workspace + Tauri v2 backend skeleton with all 18 Phase-1 crates pinned at empirically-verified versions, wired `tracing-subscriber` for stdout logging, and resolved Assumption A4 against three real gbe_fork saves before any adapter code is written.

## What Was Built

- **Workspace root.** `Cargo.toml` declares `members = ["src-tauri"]` with resolver=2, a strict release profile (`opt-level=3`, `lto="thin"`, `panic="abort"`), and shared workspace package fields. `Cargo.lock` is committed (binary-crate convention).
- **`src-tauri/` crate.** Single Cargo crate with both a binary (`hallmark`) and a library (`hallmark_lib`) that share `src/`. All Phase 1 dependencies pinned to minor versions verified via `cargo info` on 2026-05-08 — `tauri 2.11`, `tokio 1.52`, `notify 8.2`, `notify-debouncer-full 0.7` (correcting STACK.md's stale `0.5`), `rusqlite 0.39 (bundled)`, plus the rest of the stack listed in the frontmatter.
- **Tauri configuration.** `src-tauri/tauri.conf.json` declares NO windows (`app.windows: []`) and disables bundling (`bundle.active: false`) — Phase 1 is backend-only by design. Phase 2 populates windows; Phase 4 flips `bundle.active`.
- **Rust entry points.** `main.rs` is a five-line idiomatic shim into `hallmark_lib::run()`. `lib.rs` initialises tracing (RUST_LOG-driven, defaults to `hallmark_lib=info,warn`), starts `tauri::Builder::default()` with an empty `setup()` hook, and emits `tracing::info!("Hallmark starting (Phase 1 — backend only, no UI)")` at startup.
- **Domain errors.** `error.rs` defines `PathDiscoveryError` (Plan 03 will extend), `AdapterError` (Plan 04 will extend), and `StoreError` (Plan 02 will extend) — all with `thiserror::Error` derives.
- **Module stubs.** `paths.rs`, `sources/mod.rs`, `watcher/mod.rs`, `store/mod.rs` are doc-only stubs that compile cleanly and are declared from `lib.rs` so downstream plans don't trigger module-not-found errors during their own `cargo check` runs.
- **Goldberg fixture.** `tests/fixtures/goldberg/480/achievements.json` covers four canonical cases (earned-with-timestamp, unearned, second-unearned-for-diff, earned-but-`earned_time=0` per PITFALLS.md #15). Spacewar (appid 480 = official Steamworks SDK demo) is convention-safe as a fixture appid.
- **A4 resolution.** `empirical-goldberg-schema-NOTES.md` records the PowerShell scan that surfaced three real gbe_fork saves under `%APPDATA%\GSE Saves\` and confirms field names `earned`/`earned_time` match the documented schema in all three.

## Key Decisions Made

| Decision | Rationale | Alternatives Considered |
|----------|-----------|-------------------------|
| `tauri-build = "2.6"` instead of `"2.11"` | tauri-build's version track is independent of the tauri runtime; latest stable on crates.io is 2.6.1 (2.11 does not exist) — this was a Rule 3 blocking discovery during `cargo fetch`. | Pre-release `2.11.x-rc` candidates exist but the project rule is to avoid prereleases. |
| Generate a placeholder multi-layer transparent ICO | `tauri-build` hard-requires `icons/icon.ico` for Windows resource embedding; the plan's `"icon": []` did not satisfy this. A 304KB placeholder unblocks compilation without committing real branding before Phase 4. | Skipping `tauri-build` entirely is not an option — `tauri::generate_context!()` requires its output. |
| Commit `dist/index.html` placeholder | `tauri::generate_context!()` validates `frontendDist` resolves to an existing directory at proc-macro expansion time; without a stub directory the macro panics. The placeholder is whitelisted in `.gitignore`. | Stripping `frontendDist` entirely is not supported by Tauri 2.x configuration. |
| Set `bundle.targets = "all"` (with `bundle.active = false`) | `"none"` is not a valid `bundle.targets` value (valid set: `all`, `deb`, `rpm`, `appimage`, `msi`, `nsis`, `app`, `dmg`); `bundle.active = false` already prevents bundling so the targets value is dormant. | Removing the `targets` field defaults to the same set; explicit `"all"` keeps Phase 4 bundling work straightforward. |
| Resolve A4 by direct empirical inspection | Three real gbe_fork saves were available on the developer machine (`%APPDATA%\GSE Saves\1455840`, `1948280`, `2592160`), so the LOW-confidence assumption is upgraded to direct observation. | Conservative-fallback path (relying on three independent secondary sources) was unnecessary because primary observation succeeded. |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `tauri-build` version pinning corrected**
- **Found during:** Task 2, first `cargo fetch` run.
- **Issue:** Plan and RESEARCH.md both pin `tauri-build = "2.11"`, but crates.io shows latest stable is 2.6.1 — `cargo fetch` failed: `failed to select a version for the requirement tauri-build = "^2.11"`.
- **Fix:** Pinned to `2.6`. The tauri-build crate has a version track independent of the tauri runtime crate. Documented inline in `src-tauri/Cargo.toml` with a comment.
- **Files modified:** `src-tauri/Cargo.toml`
- **Commit:** `d43ef61`

**2. [Rule 3 - Blocking] `bundle.targets = "none"` is not a valid value**
- **Found during:** Task 3, first `cargo check` run.
- **Issue:** `tauri.conf.json` had `"targets": "none"` per the plan, but `tauri-build`'s validator rejects it: "invalid bundle type none, expected one of `all`, `deb`, `rpm`, `appimage`, `msi`, `nsis`, `app`, `dmg`".
- **Fix:** Set `"targets": "all"`. Bundling stays disabled because `bundle.active = false`.
- **Files modified:** `src-tauri/tauri.conf.json`
- **Commit:** `452d29b`

**3. [Rule 3 - Blocking] `tauri-build` requires a real `icons/icon.ico`**
- **Found during:** Task 3, second `cargo check` run.
- **Issue:** `tauri-build`'s build script aborts with `'icons/icon.ico' not found; required for generating a Windows Resource file during tauri-build` even when bundling is disabled.
- **Fix:** Generated a minimal multi-layer transparent placeholder ICO (16/24/32/48/64/256, 32bpp) and pointed `tauri.conf.json` `bundle.icon` at it. Real branding will replace this file in Phase 4 before bundling.
- **Files modified:** `src-tauri/icons/icon.ico` (new), `src-tauri/tauri.conf.json`
- **Commit:** `452d29b`

**4. [Rule 3 - Blocking] `tauri::generate_context!()` validates `frontendDist` exists**
- **Found during:** Task 3, third `cargo check` run.
- **Issue:** Macro panic: `The 'frontendDist' configuration is set to "../dist" but this path doesn't exist`. The plan's text said `frontendDist` was a placeholder, but the macro doesn't tolerate a missing directory.
- **Fix:** Created a tiny `dist/index.html` placeholder (with a comment explaining its Phase-1 purpose) and updated `.gitignore` to whitelist only that file inside `/dist/*`.
- **Files modified:** `dist/index.html` (new), `.gitignore`
- **Commit:** `452d29b`

**5. [Rule 1 - Cleanup] `cargo fmt` reformatted `tracing::info!` line**
- **Found during:** Task 3 verification (`cargo fmt --check`).
- **Issue:** A single `tracing::info!(...)` call exceeded the 100-column line limit.
- **Fix:** Ran `cargo fmt` once to apply the formatter; line was wrapped onto three lines. Diff is cosmetic only.
- **Files modified:** `src-tauri/src/lib.rs`
- **Commit:** `452d29b`

### Authentication Gates

None occurred during this plan.

## What Plans 02–05 Need to Fill

| Plan | Module | What it owns | Notes |
|------|--------|--------------|-------|
| 02 | `src-tauri/src/store/mod.rs` | `SqliteStore`, migrations, baseline + dedup queries | Will extend `error::StoreError` with sqlite-specific variants. |
| 02 | `src-tauri/src/sources/mod.rs` | `SourceAdapter` trait, `RawUnlockEvent`, `SourceKind` | Plan 04 adds `pub mod goldberg;` under this module. |
| 03 | `src-tauri/src/paths.rs` | Steam install registry probe, `libraryfolders.vdf`, `local_save.txt` | Will extend `error::PathDiscoveryError`. |
| 04 | `src-tauri/src/sources/goldberg.rs` (new) | Goldberg adapter — uses A4-confirmed schema `{api_name: {earned: bool, earned_time: u64}}` | Must apply `serde_json::Value` fallback per A4 NOTES.md "Conservative Fallback" — even though A4 confirmed, defense-in-depth is required. |
| 04 | `src-tauri/src/watcher/mod.rs` | `WatcherCore`, `notify-debouncer-full` driver, content-hash dedup | Hooks into `lib.rs::run()` `setup()` closure via `tokio::spawn`. |
| 05 | `src-tauri/src/watcher/dedup.rs` (new) | Cross-source dedup with TTL + `unlock_history` UNIQUE INDEX | Adds the second `[[bin]] name = "hallmark-cli"` target to `src-tauri/Cargo.toml`. |

## Threat Flags

No new threat surface beyond what is already in the plan's `<threat_model>`. The placeholder `icons/icon.ico` is a binary asset embedded into the Windows resource section of the executable; it contains only zeros and is not externally sourced. The placeholder `dist/index.html` is committed to git and contains no executable content.

## Self-Check: PASSED

- `Cargo.toml` exists at workspace root.
- `src-tauri/Cargo.toml` exists with `notify-debouncer-full = "0.7"` and `tauri = { version = "2.11", ... }`.
- `src-tauri/src/{main.rs, lib.rs, error.rs, paths.rs, sources/mod.rs, watcher/mod.rs, store/mod.rs}` all exist.
- `src-tauri/icons/icon.ico` exists (304886 bytes, MS Windows icon resource, 6 layers).
- `tests/fixtures/goldberg/480/achievements.json` exists and parses as JSON with 4 expected keys.
- `tests/fixtures/goldberg/README.md` exists.
- `.planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md` exists, contains both `earned` and `earned_time`, contains heading `## Decision for Plan 04` and `## Real Saves Inspected`.
- Commits exist: `b6076da` (Task 1), `d43ef61` (Task 2), `452d29b` (Task 3) — verified via `git log --oneline`.
- `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` returns exit 0.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` returns exit 0.
- Smoke test: `target/debug/hallmark.exe` boots, emits `Hallmark starting (Phase 1 — backend only, no UI) version="0.1.0"` plus `Tauri setup complete (no background tasks attached in Phase 1 scaffold)` to stdout, and the run loop stays alive until killed.
