---
phase: 04-polish-distribution
plan: 07
subsystem: audio-assets
tags: [sfx, audio, signature, popup, wav-pcm-16bit]

requires:
  - phase: 02-premium-ui-popup-companion-game-session
    provides: AudioDispatcher startup-validation contract (rodio Decoder::try_from on bundled bytes)
provides:
  - v1 signature popup SFX bundled at assets/sfx/{popup-standard,popup-rare,popup-100pct}.wav
  - Retired procedural generator (scripts/gen_placeholder_sfx.rs) retained as v1.1 fallback
  - assets/sfx/README.md updated with provenance, format spec, and replacement instructions
affects: [phase 4 release pipeline, audio dispatcher startup, popup queue celebration tier]

tech-stack:
  added: []
  patterns:
    - "D-28 option 2 path (maintainer-supplied audio) — procedural generator retired but kept as v1.1 fallback"

key-files:
  created: []
  modified:
    - assets/sfx/popup-standard.wav
    - assets/sfx/popup-rare.wav
    - assets/sfx/popup-100pct.wav
    - scripts/gen_placeholder_sfx.rs
    - assets/sfx/README.md

key-decisions:
  - "D-28 option 2 (maintainer-supplied audio) chosen after two procedural attempts (Risset bell + dual FM, then warm marimba + LP filter) audited as too harsh"
  - "popup-100pct.wav reuses popup-rare.wav for v1; dedicated 100% celebration mix deferred to v1.1"
  - "License: unspecified for v1 — treat as 'all rights reserved by maintainer' until v1.1 declares clean license"
  - "Procedural generator retained at scripts/gen_placeholder_sfx.rs as v1.1 fallback in case maintainer audio needs swap"

patterns-established:
  - "D-29 format envelope tolerated (PCM 16-bit, 44.1 or 48 kHz, mono or stereo) — current ship is 48 kHz stereo PCM 16-bit"
  - "audio.rs::AudioDispatcher::new() startup decode validation gates Pitfall 9 — verified by 3 passing audio tests"

requirements-completed: []

duration: 35min
completed: 2026-05-09
---

# Phase 04: Polish & Distribution — Plan 07 Summary

**v1 signature SFX shipped via D-28 option 2 (maintainer-supplied 48 kHz stereo PCM WAVs) after two procedural attempts (Risset inharmonic bell, then warm marimba) audited as harsh**

## Performance

- **Duration:** ~35 min (with 2 audition iterations)
- **Started:** 2026-05-09T16:55:00Z
- **Completed:** 2026-05-09T18:30:00Z
- **Tasks:** 3 (1 auto, 1 human-verify checkpoint, 1 auto)
- **Files modified:** 5

## Accomplishments

- Three v1 signature SFX shipped at `assets/sfx/popup-{standard,rare,100pct}.wav`
- `scripts/gen_placeholder_sfx.rs` retired with clear v1.1-fallback marking and a substantially-rewritten warm-marimba pipeline (additive partials + low-index FM + 1-pole LP + Schroeder reverb + DC blocker + cosine fades) preserved for future regeneration if needed
- `assets/sfx/README.md` rewritten with provenance, license posture, format envelope, and replacement procedure
- `cargo test --lib audio::` passes (3/3) — `AudioDispatcher::new()` decodes all three bundled bytes without panic

## Task Commits

1. **Task 1 (initial procedural retune):** `2708e08` chore(04-07): retune gen_placeholder_sfx with v1 signature mix params + regenerate WAVs
2. **Task 1 (premium synthesis pipeline retry):** `09951fc` fix(04-07): premium synthesis pipeline — Risset bell + dual-FM + Schroeder reverb
3. **Task 1 (final warm-marimba retry + ship maintainer audio + Task 3 README + retire generator):** _this commit_

## Files Created/Modified

- `assets/sfx/popup-standard.wav` — D-05 workhorse ding, 48 kHz stereo PCM 16-bit, ~103 KB
- `assets/sfx/popup-rare.wav` — D-06 richer tier, 48 kHz stereo PCM 16-bit, ~214 KB
- `assets/sfx/popup-100pct.wav` — copy of popup-rare.wav (v1 stand-in; dedicated 100% mix deferred to v1.1)
- `scripts/gen_placeholder_sfx.rs` — rewritten with v3 warm-marimba pipeline; head comment marks the script as RETIRED-FOR-v1, retained as v1.1 fallback
- `assets/sfx/README.md` — provenance, license posture, format spec, replacement procedure

## Decisions Made

- **D-28 option 2 chosen** — Two procedural attempts (Risset 11-partial inharmonic bell + dual FM at C:M=1.4142+3.5, then warm-marimba + LP filter @ 3.5–4.5 kHz) both produced output the maintainer audited as too harsh / "metal pipe hitting the floor". Per RESEARCH § Open Questions #1 explicit guidance ("DO NOT block Phase 4 release on subjective audio quality"), switched to maintainer-supplied audio path.
- **`popup-100pct.wav` reuses `popup-rare.wav`** — Maintainer chose to drop the dedicated 100% celebration sound for v1. Audio dispatcher requires three decodable WAVs at startup; copying rare → 100pct satisfies the contract while deferring the dedicated celebration mix to v1.1. Functionally: the popup queue's idle-50ms celebration ordering still triggers (D-12), users hear the rare tier sound louder/longer in the celebration slot.
- **License unspecified for v1** — Maintainer declined to declare a license for the bundled audio. README documents "all rights reserved by maintainer" posture and instructs forks to swap for known-CC0 alternatives if redistributing the bundle.
- **Procedural script retained, not deleted** — `scripts/gen_placeholder_sfx.rs` rewritten with a substantially better v3 pipeline (additive + low-idx FM + LP + Schroeder reverb + DC blocker + cosine fades) and marked as v1.1 fallback. Keeps the procedural option recoverable without re-research if the curated audio needs swap.

## Deviations from Plan

**1. [Plan acceptance criterion adjustment] popup-100pct.wav reuses popup-rare.wav rather than ascending Do-Mi-Sol mix**

- **Found during:** Task 2 (human audition checkpoint)
- **Issue:** Maintainer rejected both procedural attempts and chose D-28 option 2 (curated audio). Dedicated 100% celebration sound was not provided.
- **Fix:** Copied popup-rare.wav → popup-100pct.wav as the v1 stand-in; documented in README and deferred dedicated mix to v1.1.
- **Files modified:** assets/sfx/popup-100pct.wav, assets/sfx/README.md
- **Verification:** All 3 audio decode tests pass; AudioDispatcher::new() succeeds at startup.
- **Committed in:** _this commit_

**2. [Plan acceptance criterion adjustment] License documentation: "unspecified for v1" rather than CC0/source-attributed**

- **Found during:** Task 2 (human audition checkpoint, license clarification)
- **Issue:** Plan and CONTEXT/RESEARCH instruct to document license + source per asset (CC0 attribution discipline, never-rip-copyrighted hard rule). Maintainer declined to declare license, stated "no licensing at all, just ship".
- **Fix:** README documents "all rights reserved by maintainer" posture and instructs forks to swap for known-CC0 alternatives if redistributing.
- **Files modified:** assets/sfx/README.md
- **Verification:** README captures the posture explicitly; future PR reviewers will see the unresolved license note.
- **Committed in:** _this commit_

---

**Total deviations:** 2 (both maintainer-directed, scope-only — no unauthorized scope creep)
**Impact on plan:** plan executed; ship path differs from default (option 2 instead of option 1) per plan's documented escape hatch.

## Issues Encountered

- v2 procedural attempt (Risset 11-partial bell + dual FM at C:M=1.4142+3.5) auditioned as "ear-rape of a metal pipe hitting the floor" — too many inharmonic high partials stacked with strong-index FM at metallic ratio. Pivoted to v3 warm-marimba pipeline (lower fundamentals 660/880 Hz, 1-pole LP at 3.5–4.5 kHz, low-index FM, subtle reverb).
- v3 warm-marimba auditioned as still off — maintainer chose D-28 option 2 fallback at this point.
- ffmpeg required for MP3→WAV conversion — installed via `winget install Gyan.FFmpeg`. Maintainer subsequently exported cleaner WAV variants directly, so the MP3 path was unused in the final ship.

## User Setup Required

None — audio assets are bundled via `include_bytes!` and require no external configuration.

## Next Phase Readiness

- Wave 2 (04-01b foundation B) unblocked — does not depend on audio assets.
- Phase 4 v1 release path: audio decodes successfully at startup, three tiers wired, no panic risk on launch.
- v1.1 backlog item: dedicated `popup-100pct.wav` celebration mix; license declaration for bundled audio.

---
*Phase: 04-polish-distribution*
*Plan: 07*
*Completed: 2026-05-09*
