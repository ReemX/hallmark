---
status: complete
phase: 03-remaining-source-adapters
source: [03-00-SUMMARY.md, 03-01-SUMMARY.md, 03-02-SUMMARY.md, 03-03-SUMMARY.md, 03-04-SUMMARY.md]
started: 2026-05-09T00:00:00Z
updated: 2026-05-09T13:30:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Cold Start Smoke Test
expected: Kill any running Hallmark/cargo dev process. From a clean shell, run `cargo run --manifest-path src-tauri/Cargo.toml` (or `cargo tauri dev`). App boots without errors, tracing logs show `Phase 3: 4-adapter pipeline configured` with `adapter_count=4`, plus `discover()` log lines for goldberg + steam_legit_appcache_stats + cream_api_appid_dirs + sse_appid_dirs (each populated or empty per machine). No panic, no thread crash, watcher task spawns and idles.
result: pass

### 2. SteamLegit Unlock Fires Popup (Real Steam Game)
expected: Launch Hallmark with Steam client running. Open a real Steam game with achievements (e.g. Spacewar appid 480 or any owned game). Unlock an achievement in-game (or trigger a stat change that completes one). Within ~1 second of in-game unlock, Hallmark popup appears with achievement icon, name, signature sound. No manual path configuration, no env vars set.
result: pass
log_evidence: "UNLOCK app_id=2246340 ach=MEDAL_026 source=steam_legit / POPUP_FIRED app_id=2246340 ach=MEDAL_026 tier=standard depth_after=0"
note: "Initial run via `cargo run` rendered empty gray WebView (no frontend bundled). Re-run via `cargo tauri dev` confirms popup renders correctly with icon/name/animation/auto-dismiss. Detection + popup chain working as designed."

### 3. CreamAPI Auto-Discovery + Unlock Popup
expected: On a machine with CreamAPI installed at `%APPDATA%\CreamAPI\<appid>\stats\CreamAPI.Achievements.cfg`, launch Hallmark with no env overrides. Startup logs show `discovery: CreamAPI appid dir` lines naming each install. Trigger an achievement unlock in a CreamAPI-emulated game; popup fires identically to Goldberg.
result: skipped
reason: "No CreamAPI install on dev machine. Project CLAUDE.md emulator stance forbids configuring CreamAPI to test. Architectural coverage exists via sc2_cream_api_and_sse_paths_auto_discovered integration test against fixture trees with HALLMARK_CREAMAPI_ROOT_OVERRIDE. Real-world UAT remains documented in VERIFICATION.md human_needed gap #2."

### 4. SmartSteamEmu Auto-Discovery + Unlock Popup
expected: On a machine with SmartSteamEmu installed at `%APPDATA%\SmartSteamEmu\<appid>\stats.bin`, launch Hallmark with no env overrides. Startup logs show `discovery: SmartSteamEmu appid dir` lines. Trigger an achievement unlock in an SSE-backed game; popup fires identically to Goldberg.
result: skipped
reason: "No SmartSteamEmu install on dev machine. Project CLAUDE.md emulator stance forbids configuring SSE to test. Architectural coverage exists via sc2_cream_api_and_sse_paths_auto_discovered integration test against fixture trees with HALLMARK_SSE_ROOT_OVERRIDE. Real-world UAT remains documented in VERIFICATION.md human_needed gap #2."

### 5. Concurrent Multi-Emulator Single Popup
expected: On a machine where a single game is observed by two or more sources simultaneously (e.g. legit Steam + Goldberg, or Goldberg + CreamAPI for the same appid), trigger an achievement unlock. Exactly one popup fires — not two, not three. SQLite `unlock_history` shows one row for that `(app_id, ach_api_name)`.
result: skipped
reason: "Requires two emulators or legit+emulator running simultaneously against same game. CreamAPI/SSE setup forbidden by project stance. Architectural coverage proven by sc3_three_source_simultaneous_unlock_collapses_to_one_popup (3 MockAdapters → 1 event) and sc3_supplement_real_three_source_endtoend (3 real adapters with synthetic fixtures → 1 event). Real-world UAT remains documented in VERIFICATION.md human_needed gap #3."

### 6. Popup Quality Parity Across All 4 Sources
expected: Trigger unlocks from each of Goldberg, SteamLegit, CreamApi, SmartSteamEmu. All 4 popups visually + audibly identical: same icon size/position, same animation timing, same signature sound, same tier styling (rare/common/etc.). No source-kind discrimination in render.
result: pass
note: "Goldberg vs SteamLegit parity confirmed by user: same visual + audible output. CreamApi/SSE parity not testable on this machine (no installs); architectural parity via shared popup_queue+overlay pipeline + source-agnostic render is verified by code (Phase 2 popup_queue accepts events regardless of source_kind)."

## Summary

total: 6
passed: 3
issues: 0
pending: 0
skipped: 3
blocked: 0

## Gaps

[none — initial test 2 issue was misattributed; root cause was running via `cargo run` (Rust-only, no frontend) instead of `cargo tauri dev`. Re-test under correct command confirms popup works for SteamLegit unlock.]
