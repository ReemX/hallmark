# Phase 4: Polish & Distribution - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-09
**Phase:** 4-Polish & Distribution
**Areas discussed:** Tray + settings surface, First-run wizard UX, Update prompt UX + channel, Code signing + signature SFX assets

---

## Tray + settings surface

### Q1: Tray menu structure

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal: Show companion / Fire test / Quit | 3 items only; settings in separate window | |
| Full: Show companion / Fire test / Settings… / Start with Windows / Quit | Inline checkable autostart toggle + Settings… opens window | ✓ |
| Tray-only no settings window | Everything via tray; no dedicated settings window | |

**User's choice:** Full tray menu with inline `☑ Start with Windows`.
**Notes:** User selected the option preview directly — that ASCII layout is now the locked reference for D-01.

### Q2: Settings window contents

| Option | Description | Selected |
|--------|-------------|----------|
| Detected paths + check-for-updates + about | Read-only panels, no knobs | ✓ |
| Above + Start-with-Windows toggle (duplicated from tray) | Belt-and-suspenders autostart access | |
| Above + diagnostic log viewer | Tracing-tail ring buffer for issue triage | |

**User's choice:** Read-only Detected paths + Updates + About only.
**Notes:** Diagnostic log viewer noted as deferred (could surface after first wave of bug reports). Start-with-Windows stays tray-only — registry is source of truth.

### Q3: Test popup trigger inject point

| Option | Description | Selected |
|--------|-------------|----------|
| Synthesize RawUnlockEvent at sources→dedup boundary | Hits real dedup, schema, queue, audio, monitor; not the watcher | (Claude picked) ✓ |
| Inject directly into popup_queue | Bypasses dedup + schema; fastest feedback | |
| Touch a real file fixture | Most honest including watcher; slowest, fragile | |

**User's choice:** "you decide based on research" → Claude picked option 1.
**Notes:** ROADMAP SC#1 says "fires through the full pipeline". Synthesizing at the adapter→dedup boundary covers everything except the kernel-level watcher event itself, which is OS-level and fundamentally not something a self-test can validate (real game unlocks already validate it). Cleanest cost/coverage tradeoff.

### Q4: POL-02 start-with-Windows registry behavior

| Option | Description | Selected |
|--------|-------------|----------|
| HKCU Run, points to installed exe, no flags | App starts in normal mode; brief window flash | |
| HKCU Run, with --silent flag | Tray-only on autostart; user opens companion manually | (Claude picked) ✓ |
| HKCU Run, --silent + delayed-start key | Above + ~30s delay to reduce login I/O | |

**User's choice:** "you decide what is most common for these types of programs" → Claude picked option 2.
**Notes:** Discord, Slack, Steam, OBS — every tray utility uses `--silent`-style autostart. Brief window flash on login is unprofessional for a tray app.

---

## First-run wizard UX

### Q1: Wizard surface

| Option | Description | Selected |
|--------|-------------|----------|
| Modal dialog over companion window on first launch | Companion opens; modal sheet drops over it | |
| Standalone wizard window (own borderless card) | Dedicated 'Welcome' window separate from companion | (Claude picked) ✓ |
| Minimal: tray notification + Settings panel does the heavy lifting | Toast → Settings 'Detected sources' | |

**User's choice:** "you decide" → Claude picked option 2.
**Notes:** First launch is a one-time moment that deserves dedicated treatment. ROADMAP SC#5 demands "immediate actionable message rather than silent failure" — a toast is too easily dismissed for that bar.

### Q2: 0-paths case copy

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit 'no sources found' + actionable hints | Lists what was scanned + brief explainer; no setup help | ✓ |
| 'No sources yet' + Fire test popup as immediate gratification | Above + prominent test-popup CTA | |
| Generic 'still scanning' + retry | Minimal verbiage; no scan-paths leaked | |

**User's choice:** Explicit no-sources framing.
**Notes:** Honesty first — users diagnose better when they see "Steam libraryfolders.vdf — not found". Stays passive (no install/configure instructions for emulators).

### Q3: Source labeling

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit: 'Steam', 'Goldberg', 'CreamAPI', 'SmartSteamEmu' | Direct names; honest | ✓ |
| Two-bucket: 'Steam' + 'Other detected sources' | Genericize emulator branding | |
| All as 'Achievement sources — N detected' | Just a count | |

**User's choice:** Explicit names.
**Notes:** Naming is not endorsing or recommending. PROJECT.md passive-detection rule prohibits installing/configuring emulators, not naming them.

### Q4: Wizard re-fire policy

| Option | Description | Selected |
|--------|-------------|----------|
| Once-ever, on first launch after install | SQLite flag set on dismissal; never re-shown automatically | |
| Once-ever, plus re-trigger if zero paths still detected | Re-fires until at least 1 path found, then latches | (Claude picked) ✓ |
| On every launch until at least 1 path detected | No flag; always shows on 0-paths | |

**User's choice:** "you decide" → Claude picked option 2.
**Notes:** Handles the install-Hallmark-before-first-game case naturally. Once any path is detected, latch the flag forever. Settings → Rescan handles ad-hoc rescans after that.

---

## Update prompt UX + channel

### Q1: Update notification surface

| Option | Description | Selected |
|--------|-------------|----------|
| Toast on tray + badge until acknowledged | Windows toast; click → modal | |
| Modal on next companion-window open | Modal sheet next time companion appears | (Claude picked) ✓ |
| Settings-only — manual check or red dot in tray | Very passive; users may never update | |

**User's choice:** "you decide best way to keep users updated" → Claude picked option 2.
**Notes:** Companion opens at game launch — high engagement, never during gameplay (companion is hidden then), higher conversion than easily-dismissed toast. Settings-only too passive for security updates even on hobby OSS.

### Q2: Install flow

| Option | Description | Selected |
|--------|-------------|----------|
| Download, prompt 'Restart Hallmark now / later' | User controls timing | |
| Download + immediate Hallmark auto-restart | Fastest; brief tray/companion blip | ✓ |
| Download + activate on next launch | Zero disruption; slowest activation | |

**User's choice:** Option 2 (immediate auto-restart).
**Notes:** User asked clarifying question — confirmed "restart" = Hallmark process only, never the PC. Reasoning: ~1-min update, user explicitly triggered it, no value in deferring.

### Q3: Update channel

| Option | Description | Selected |
|--------|-------------|----------|
| Stable only for v1 | Single latest.json | ✓ |
| Stable + prerelease opt-in toggle in Settings | Two latest.json files | |
| Stable only + 'Check for prerelease' button | One-shot manual peek | |

**User's choice:** Stable only for v1.
**Notes:** Direct user quote: "we will never have pre-release for this scope unless project starts gaining open-source traction". Deferred to v2 contingent on community size.

### Q4: Updater signing keypair location

| Option | Description | Selected |
|--------|-------------|----------|
| GitHub Actions secret only — generated once, never on local disk | CI-only signing; smallest blast radius | (Claude picked) ✓ |
| Keypair lives in user's local secrets store + CI mirror | Allows local signed test builds | |
| Defer signing key for v1 — ship updater unsigned | tauri-plugin-updater 2.x requires signing — effectively defers DIST-02 | |

**User's choice:** "you decide based on phase research" → Claude picked option 1.
**Notes:** All releases come from CI per DIST-03. No reason to keep a local copy of the private key. Researcher must validate `tauri signer generate` flow + tauri-action env var contract before this is locked in code.

---

## Code signing + signature SFX assets

### Q1: NSIS code signing

| Option | Description | Selected |
|--------|-------------|----------|
| Unsigned installer — accept SmartScreen warning | Free; README documents workaround | |
| Self-signed cert | Free but worse UX | |
| Defer signing, document signed-build path for later | Ships unsigned + CI placeholders for future cert | ✓ |

**User's choice:** Defer with documented path for future.
**Notes:** Hobby OSS pattern (Notepad++, OBS-launcher, etc). README needs SmartScreen-warning screenshot + "More info → Run anyway" instructions.

### Q2: Installer scope

| Option | Description | Selected |
|--------|-------------|----------|
| Per-user install (no admin), to %LOCALAPPDATA%\Hallmark | No UAC; matches Discord/Slack | (Claude picked) ✓ |
| Per-machine install (admin), to Program Files | UAC every install/update | |
| User choice at install time | Adds wizard page; complex | |

**User's choice:** "what do you think? most people only have 1 user on their pc" → Claude picked option 1.
**Notes:** User reasoning correct — single-user PCs are the norm. Updater works without admin in per-user mode, which is critical for the auto-update flow.

### Q3: Portable .zip behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Self-contained: hallmark.exe + assets, runs from any folder, writes to %APPDATA% | Standard Tauri portable; updater disabled in portable mode | (Claude picked) ✓ |
| Truly portable: writes everything beside the exe | Power-user friendly; explicit code path required | |
| Skip portable .zip for v1, ship installer only | Roadmap deviation (DIST-01 + SC#3 require both) | |

**User's choice:** "you pick, not sure I even want portable release" → Claude picked option 1.
**Notes:** DIST-01 + SC#3 both require portable. Standard portable is the lowest-complexity satisfaction of the requirement. Truly-portable is a deferred power-user request.

### Q4: Signature SFX direction

| Option | Description | Selected |
|--------|-------------|----------|
| Tune the procedural script — refine gen_sfx params | Keep gen_sfx pipeline; iterate; zero licensing risk | (preference 1) |
| Hand-crafted in DAW — produced final WAVs | Best timbral control; requires audio chops | |
| Royalty-free pack curation + layer in DAW | License attribution required; CC0-OSS-compatible verification needed | (preference 2) |

**User's choice:** Free-text — "want most polished feel possible without outside contractor; depends on phase research; open to procedural OR royalty-free OR ripping if it gets better results".
**Notes:** Marked as RESEARCH FLAG (D-28), not locked. Claude redirected the rip-copyrighted-sounds option — for an OSS GitHub release, redistributing copyrighted SFX (PS5/Xbox/etc) is a DMCA-strike + contributor-legal-risk hard NO. CC0 royalty-free is fine if license verification confirms OSS-redistribution. Researcher to investigate procedural state-of-the-art (SuperCollider/Faust/Tone.js) and CC0 pack inventory (freesound.org CC0 filter, Pixabay terms) and recommend the path that gets closest to PS5-grade with zero licensing risk.

---

## Claude's Discretion

- Test-popup inject point (Q3 of Tray area) — picked sources→dedup boundary.
- Autostart registry pattern (Q4 of Tray area) — picked HKCU Run + `--silent` flag.
- First-run wizard surface (Q1 of Wizard area) — picked standalone borderless card.
- Wizard re-fire policy (Q4 of Wizard area) — picked once-ever-with-0-paths-retrigger.
- Update notification surface (Q1 of Update area) — picked modal-on-companion-open.
- Updater signing keypair location (Q4 of Update area) — picked GitHub Actions secret only.
- Installer scope (Q2 of Signing/SFX area) — picked per-user install.
- Portable .zip behavior (Q3 of Signing/SFX area) — picked standard self-contained portable.
- Tray icon glyph design — Claude/researcher within Hallmark monochrome theme.
- "Updates" panel last-checked timestamp display — minor polish, Claude's call.
- First-run wizard exact copy — Claude writes within established voice.
- About panel exact links + license SPDX — Claude derives from repo state.
- NSIS installer wizard pages (welcome / install dir / install / finish) — tauri-action defaults unless friction emerges.
- Portable-mode detection heuristic (folder-writable vs. `--portable` flag) — researcher's call.
- Test-popup rarity lookup behavior (cached/zero vs. live Web API) — Claude/planner.

## Deferred Ideas

- Diagnostic log viewer in Settings (in-memory tracing tail) — deferred to v1.1.
- Update-channel selector (stable / prerelease toggle) — deferred to v2 contingent on traction.
- Auto-update on/off toggle — deferred to v1.1 after observing modal-flow friction.
- Code signing + paid cert — deferred indefinitely (cost vs. hobby pace).
- Truly-portable mode (state-beside-exe) — deferred to power-user request.
- Companion size/position reset button in Settings — deferred to first user request.
- Telemetry / crash reporting — explicitly OUT OF SCOPE per PROJECT.md, not deferred.
