---
phase: 03
slug: remaining-source-adapters
status: verified
threats_open: 0
asvs_level: 1
created: 2026-05-09
---

# Phase 03 — Security: Remaining Source Adapters

**Audit date:** 2026-05-09
**ASVS level:** L1
**Block-on policy:** high (open mitigations block ship)
**Mode:** verify-mitigations (threat register authored at plan-time; not retroactive STRIDE)

---

## Outcome

**SECURED — 27/27 threats CLOSED (16 mitigate verified in code, 11 accepted with rationale).**

All declared `mitigate` dispositions are present in the implementation files cited below. All `accept` dispositions have sound rationale documented in the originating plan and are recorded in the accepted-risks log.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| dev-machine filesystem → planner | Empirical PowerShell scan reads from privileged user path; no untrusted input crosses | local file paths, hex bytes |
| filesystem (`appcache/stats`) → SteamLegitAdapter | Untrusted binary blobs from Steam's writes; format may evolve | binary VDF |
| Windows registry HKCU → discover_paths | Local-user-controlled keys; could be empty/missing | Steam install path, user IDs |
| filesystem (`%APPDATA%\CreamAPI\*`) → CreamApiAdapter | Untrusted user/scene-release-controlled INI files | INI text |
| filesystem (`%APPDATA%\SmartSteamEmu\*`) → SseAdapter | Untrusted user/scene-release-controlled binary blobs | binary stats |
| Goldberg companion file (`%APPDATA%\GSE Saves\<appid>\`) | Same trust as Goldberg adapter — locally controlled but unvalidated | JSON |
| 4 adapters → run_watcher (`Vec<Arc<dyn SourceAdapter>>`) | Internal trust; trait + Send + Sync + 'static contract is the boundary | RawUnlockEvent |
| Test fixtures → integration_phase3.rs | Test-side fixtures only; no production attack surface | synthetic blobs |

---

## Threat Verification Summary

| Plan | Threat IDs | Mitigate (verified) | Accept (logged) |
|------|------------|---------------------|------------------|
| 03-00 | T-30-T1, T-30-I1, T-30-D1, T-30-S1 | 2 | 2 |
| 03-01 | T-31-T1, T-31-T2, T-31-D1, T-31-D2, T-31-S1, T-31-I1 | 5 | 1 |
| 03-02 | T-32-T1, T-32-D1, T-32-D2, T-32-T2, T-32-S1, T-32-I1 | 4 | 2 |
| 03-03 | T-33-T1, T-33-T2, T-33-D1, T-33-S1, T-33-S2, T-33-I1 | 4 | 2 |
| 03-04 | T-34-T1, T-34-D1, T-34-S1, T-34-I1, T-34-R1 | 1 | 4 |
| **Total** | **27** | **16** | **11** |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-30-T1 | Tampering | empirical-vdf-NOTES.md content | mitigate | NOTES embeds verbatim PowerShell stdout + 32-byte hex dump (`empirical-vdf-NOTES.md:11-31`) | closed |
| T-30-I1 | Information disclosure | NOTES contains user paths + Steam user-id | accept | Local-only project; user runs planner against own machine | closed |
| T-30-D1 | DoS | byteorder/crc32fast supply chain | accept | Top-of-class single-purpose crates pinned to current stable | closed |
| T-30-S1 | Spoofing | Cargo.toml dependency typosquat | mitigate | Crate names locked: `byteorder = "1.5"` + `crc32fast = "1.4"` (`Cargo.toml:44, 46`) | closed |
| T-31-T1 | Tampering | UserGameStats binary VDF | mitigate | `parse_binary_vdf` returns `anyhow::Result`; cstr cap 1024 B; `on_file_changed` warn+Ok on parse fail (`vdf_binary.rs:57-74,161-163`, `steam_legit.rs:484-490`) | closed |
| T-31-T2 | Tampering | UserGameStatsSchema binary VDF | mitigate | Same defensive parsing; missing schema → `steam_stat_<s>_<b>` placeholder (`steam_legit.rs:200-207,414-421,506`) | closed |
| T-31-D1 | DoS | huge appcache/stats with thousands of files | mitigate | Flat `read_dir`, no recursive walk; per-file bounded by parser limits (`steam_legit.rs:376-394`) | closed |
| T-31-D2 | DoS | maliciously deep VDF nesting | mitigate | `MAX_RECURSION_DEPTH = 16` + `recursion_depth_bounded` test (`vdf_binary.rs:28,76-79,261-271`) | closed |
| T-31-S1 | Spoofing | wrong user_id files trigger event | mitigate | user_id filter against registry-discovered set (`steam_legit.rs:388-391,451-454`) | closed |
| T-31-I1 | Information disclosure | tracing logs include user paths + user_id | accept | Local stdout only | closed |
| T-32-T1 | Tampering | INI file content | mitigate | Pure parser; malformed lines silently skipped via `split_once('=')`; no `unwrap()` (`cream_api.rs:92-139`) | closed |
| T-32-D1 | DoS | huge INI file | mitigate | `read_to_string` bounded by file size; SHA-256 short-circuit prevents re-parse (`cream_api.rs:255-263,310-329`) | closed |
| T-32-D2 | DoS | INI file with 100k sections | accept | Steam max ~5000 achievements; pathological files locally-controlled (attacker = user) | closed |
| T-32-T2 | Tampering | section name with newline injection | mitigate | `text.lines()` splits before capture; WR-08 rejects empty/whitespace section names (`cream_api.rs:96,109-126`) | closed |
| T-32-S1 | Spoofing | `cream_api.cfg` masquerading | mitigate | Filename guard `file_name == "CreamAPI.Achievements.cfg"` BEFORE I/O (`cream_api.rs:224-231`, const `:39`) | closed |
| T-32-I1 | Information disclosure | tracing logs include user paths + appids | accept | Local stdout only | closed |
| T-33-T1 | Tampering | stats.bin declares `count = i32::MAX` | mitigate | `count = min(declared as usize, (bytes.len()-4)/24)`; negative→0; warn-log when capped (`sse.rs:119-129`) | closed |
| T-33-T2 | Tampering | stats.bin truncated mid-record | mitigate | Loop checks `off + 24 > bytes.len()` and `break` (`sse.rs:133-136`) | closed |
| T-33-D1 | DoS | maliciously huge stats.bin | mitigate | `Vec::with_capacity(count)` with capped count; SHA-256 short-circuit (`sse.rs:131,343-351`) | closed |
| T-33-S1 | Spoofing | `notstats.bin` masquerading | mitigate | Filename guard `file_name == "stats.bin"` BEFORE I/O (`sse.rs:312-319`, const `:54`) | closed |
| T-33-S2 | Spoofing | CRC collision producing wrong api_name | accept | CRC32 collision ~2^-32; same CRC → same baseline key → no false event | closed |
| T-33-I1 | Information disclosure | tracing logs include user paths + appids | accept | Local stdout only | closed |
| T-34-T1 | Tampering | Adapter ordering in `adapters` Vec | accept | WatcherCore dispatches to ALL matching adapters; CrossSourceDedup keys on `(app_id, ach_api_name)` (`watcher/dedup.rs:48-54`) | closed |
| T-34-D1 | DoS | 4 adapters × N watch paths | accept | 4 × ~50 = 200 paths well within debouncer limits; 500 ms debounce + per-adapter SHA-256 short-circuit | closed |
| T-34-S1 | Spoofing | adapter watch-path overlap | mitigate | WR-09 startup overlap detection logs `tracing::error!`; dispatch forwards to ALL matching adapters (`watcher/mod.rs:92-113,144-159`) | closed |
| T-34-I1 | Information disclosure | aggregated tracing logs from 4 adapters | accept | Local stdout only | closed |
| T-34-R1 | Repudiation | per-adapter dedup TTL gap | accept | Cross-adapter simultaneity sub-second in practice; SQLite UNIQUE INDEX `idx_unlock_dedup` backstop (`migrations/001_initial.sql:24`) | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-30-1 | T-30-I1 | NOTES contains user paths + Steam user-id. Local-only project; same posture as Phase 1 empirical-goldberg-schema-NOTES.md. | ReemX | 2026-05-09 |
| AR-30-2 | T-30-D1 | byteorder + crc32fast supply chain. Top-of-class single-purpose crates pinned to current stable. | ReemX | 2026-05-09 |
| AR-31-1 | T-31-I1 | tracing logs include user paths + user_id. Local stdout only; same posture as Phase 1. | ReemX | 2026-05-09 |
| AR-32-1 | T-32-D2 | INI file with 100k sections. Steam max ~5000 achievements; attacker = user. | ReemX | 2026-05-09 |
| AR-32-2 | T-32-I1 | tracing logs include user paths + appids. Local stdout only. | ReemX | 2026-05-09 |
| AR-33-1 | T-33-S2 | CRC32 collision ~2^-32. Wrong-but-stable display name; never fires false event. | ReemX | 2026-05-09 |
| AR-33-2 | T-33-I1 | tracing logs include user paths + appids. Local stdout only. | ReemX | 2026-05-09 |
| AR-34-1 | T-34-T1 | Adapter ordering in Vec. WatcherCore dispatches to ALL matching adapters; CrossSourceDedup keys on `(app_id, ach_api_name)`. | ReemX | 2026-05-09 |
| AR-34-2 | T-34-D1 | 4 × ~50 paths well within debouncer practical limits; debounce + SHA-256 short-circuit bound burst. | ReemX | 2026-05-09 |
| AR-34-3 | T-34-I1 | Aggregated tracing logs from 4 adapters. Local stdout only. | ReemX | 2026-05-09 |
| AR-34-4 | T-34-R1 | 10 s per-adapter TTL; cross-adapter simultaneity sub-second; SQLite UNIQUE INDEX `idx_unlock_dedup` backstop. | ReemX | 2026-05-09 |

*Accepted risks do not resurface in future audit runs.*

---

## Unregistered Flags

None. SUMMARY files (03-00..03-04) do not contain a `## Threat Flags` section, indicating executors did not detect new attack surface beyond the plan-time threat register.

---

## Audit Methodology Notes

- **Verification by grep + file inspection.** Each `mitigate` threat verified by locating the declared mitigation pattern at the file:line cited above. No mitigation marked CLOSED on documentation alone.
- **`accept` rationale review.** Each `accept` reviewed against the project's stated trust model (local-only Windows utility; attacker = user). All rationales consistent with that model.
- **Implementation files were READ-ONLY** during audit. No source files modified.
- **Cross-references** to Phase 1 mitigations (CrossSourceDedup, WR-09, SQLite UNIQUE INDEX) spot-verified at cited paths.
- **Test coverage** of T-31-D2 (`recursion_depth_bounded`), T-33-T1 (`parse_sse_stats_caps_count_to_file_size`), T-32-T2 (`parse_creamapi_state_rejects_empty_section_names`), T-34-S1 (`sc3_three_source_simultaneous_unlock_collapses_to_one_popup`) confirmed via SUMMARY files + code inspection.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-05-09 | 27 | 27 | 0 | gsd-security-auditor (sonnet) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter
