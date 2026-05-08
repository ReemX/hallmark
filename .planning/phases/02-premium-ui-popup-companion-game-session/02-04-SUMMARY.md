---
phase: 02-premium-ui-popup-companion-game-session
plan: "04"
subsystem: audio
tags: [rodio, wasapi, sfx, audio, wav, include_bytes, tier]

# Dependency graph
requires:
  - phase: 02-premium-ui-popup-companion-game-session
    plan: "01"
    provides: "audio.rs stub declared in lib.rs; rodio 0.22 dep in Cargo.toml"
provides:
  - "AudioDispatcher service: DeviceSinkBuilder::open_default_sink() + Mixer + pre-loaded SFX bytes"
  - "Tier enum (Standard/Rare/Completion) mapping to three WAV assets"
  - "play(tier) non-blocking via mixer.add() for concurrent layered audio"
  - "Three placeholder WAV files: popup-standard.wav (~900ms -8dBFS), popup-rare.wav (~1100ms -5dBFS), popup-100pct.wav (~1800ms -4dBFS)"
  - "scripts/gen_placeholder_sfx.rs for reproducible WAV re-generation"
affects:
  - 02-05 (popup_queue calls audio.play(tier) once per popup fire)
  - 02-07 (integration tests exercise AudioDispatcher::new() + play() on real device)
  - phase-04 (Phase 4 polish replaces placeholder WAVs with real signature mix)

# Tech tracking
tech-stack:
  added:
    - "rodio 0.22 mixer API: DeviceSinkBuilder::open_default_sink() → MixerDeviceSink; Mixer::add() for concurrent layered playback"
    - "rodio::mixer::Mixer (not re-exported at rodio root — must import from rodio::mixer::Mixer)"
    - "Decoder::try_from(Cursor<Vec<u8>>) for in-memory WAV decode without file I/O"
  patterns:
    - "Hold MixerDeviceSink in struct field named _stream for process lifetime (drop-silencing anti-pattern prevention)"
    - "Store SFX as Arc<Vec<u8>> for cheap per-play clone without file I/O"
    - "Validate bundled asset integrity at AudioDispatcher::new() before first popup fires"
    - "Return anyhow::Result from play() so audio failure is logged but does not drop visual popup"
    - "include_bytes! compile-time asset bundling for hermetic binary (no runtime file dependency)"

key-files:
  created:
    - assets/sfx/popup-standard.wav
    - assets/sfx/popup-rare.wav
    - assets/sfx/popup-100pct.wav
    - assets/sfx/README.md
    - scripts/gen_placeholder_sfx.rs
  modified:
    - src-tauri/src/audio.rs (replaced stub with full AudioDispatcher + Tier implementation)
    - .gitignore (added gen_sfx.exe + gen_sfx exclusions)

key-decisions:
  - "rodio::mixer::Mixer not re-exported at rodio top-level — import via rodio::mixer::Mixer (Rule 1 auto-fix during execution)"
  - "convert_samples::<f32>() does not exist in rodio 0.22 — mixer.add() accepts any Source directly with auto sample-rate/channel conversion (Rule 1 auto-fix)"
  - "Placeholder WAV synthesis committed for Phase 2 unblocking; real signature mix deferred to Phase 4 polish (W-9)"
  - "WASAPI shared-mode latency measurement still needed before locking audio library choice (kira fallback per RESEARCH.md still open)"

patterns-established:
  - "AudioDispatcher is a single process-lifetime struct; construct once in setup(), share via Arc"
  - "Tier enum is the IPC boundary between popup_queue and audio — popup_queue decides tier, audio plays it"

requirements-completed: [POPUP-06]

# Metrics
duration: ~15min
completed: 2026-05-08
---

# Phase 2 Plan 04: AudioDispatcher + Signature SFX Asset Bundle Summary

**rodio 0.22 AudioDispatcher with DeviceSinkBuilder::open_default_sink(), three placeholder WAV assets bundled via include_bytes!, and non-blocking mixer.add() play() returning anyhow::Result for fault-tolerant popup audio dispatch**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-05-08T14:12Z
- **Completed:** 2026-05-08T14:15Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Generated three placeholder WAV files satisfying CONTEXT.md D-05/D-06/D-12 timing + dBFS specs; validated RIFF/WAVE headers and rodio decode contract
- Implemented `AudioDispatcher` with rodio 0.22 API (`DeviceSinkBuilder::open_default_sink()`, `Mixer::add()`); holds `MixerDeviceSink` for process lifetime per RESEARCH.md anti-pattern
- All three SFX bundled via `include_bytes!` and validated at `AudioDispatcher::new()` construction — T-02-23 (bundled asset corruption) mitigation complete
- 3 unit tests pass: RIFF magic bytes, rodio decoder, tier enum distinctness — no audio device required at test time

## Task Commits

Each task was committed atomically:

1. **Task 1: Bundle three SFX WAV files** - `5315fc9` (feat)
2. **Task 2: AudioDispatcher service (audio.rs)** - `4cfce6f` (feat)

## Files Created/Modified

- `assets/sfx/popup-standard.wav` — 79,424 bytes, 44.1kHz 16-bit mono, ~900ms, peak ~-8dBFS placeholder
- `assets/sfx/popup-rare.wav` — 97,064 bytes, 44.1kHz 16-bit mono, ~1100ms, peak ~-5dBFS placeholder
- `assets/sfx/popup-100pct.wav` — 158,804 bytes, 44.1kHz 16-bit mono, ~1800ms, 4-layer placeholder
- `assets/sfx/README.md` — documents placeholder status + Phase 4 replacement instructions
- `scripts/gen_placeholder_sfx.rs` — deterministic WAV synthesizer for reproducible regeneration
- `src-tauri/src/audio.rs` — replaced stub with AudioDispatcher + Tier enum + 3 unit tests
- `.gitignore` — added gen_sfx.exe + gen_sfx exclusions

## Decisions Made

- `rodio::mixer::Mixer` is not re-exported from the `rodio` top-level crate root (despite appearing so in rustdoc's top-level struct list). Must import as `use rodio::mixer::Mixer`. (Rule 1 auto-fix)
- `convert_samples::<f32>()` does not exist in rodio 0.22. The `Mixer::add()` method accepts any `Source + Send + 'static` directly, with automatic sample-rate and channel conversion. Removed the call; `self.mixer.add(decoder)` compiles and is correct.
- Placeholder WAVs synthesized deterministically from sine waves — satisfies the rodio decode contract for Phase 2 end-to-end development; Phase 4 (W-9) must replace with real signature mix before public release.
- WASAPI latency measurement deferred — kira fallback still open per RESEARCH.md research flag and STATE.md blocker.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `rodio::Mixer` not in rodio root; use `rodio::mixer::Mixer`**
- **Found during:** Task 2 (AudioDispatcher implementation)
- **Issue:** The plan's import `use rodio::{..., Mixer, ...}` fails with `no 'Mixer' in the root` — despite docs showing it as a top-level struct, it requires `rodio::mixer::Mixer`.
- **Fix:** Split imports: `use rodio::mixer::Mixer; use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink};`
- **Files modified:** `src-tauri/src/audio.rs`
- **Verification:** `cargo test -p hallmark audio::` passes (3 tests)
- **Committed in:** `4cfce6f` (Task 2 commit)

**2. [Rule 1 - Bug] `convert_samples::<f32>()` does not exist in rodio 0.22**
- **Found during:** Task 2 (AudioDispatcher implementation)
- **Issue:** The plan calls `decoder.convert_samples::<f32>()` but this method does not exist in rodio 0.22. Compiler suggests `into_sample` but that's a per-sample scalar conversion, not what's needed.
- **Fix:** Removed the conversion call entirely — `Mixer::add()` accepts `Source + Send + 'static` directly with auto conversion. Changed to `self.mixer.add(decoder)`.
- **Files modified:** `src-tauri/src/audio.rs`
- **Verification:** `cargo build -p hallmark` exits 0; tests pass
- **Committed in:** `4cfce6f` (Task 2 commit)

**3. [Rule 1 - Bug] Removed unused `Source` import**
- **Found during:** Task 2 — compiler `unused_imports` warning after fixing `convert_samples`
- **Fix:** Removed `Source` from the use statement (not needed since `mixer.add()` handles type coercion internally)
- **Files modified:** `src-tauri/src/audio.rs`
- **Committed in:** `4cfce6f` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (all Rule 1 — plan template used rodio API surface that differs from actual 0.22 implementation)
**Impact on plan:** All auto-fixes were correctness-required. Behavior is identical to plan intent: non-blocking layered mixer.add(). No scope creep.

## Known Stubs

- `assets/sfx/popup-standard.wav`, `popup-rare.wav`, `popup-100pct.wav` — **Intentional placeholder synthesis** (see `assets/sfx/README.md`). They satisfy the rodio decode contract for Phase 2 end-to-end testing but are NOT the signature sound. Phase 4 polish (W-9) must replace with the real mix before public release. The `AudioDispatcher` itself is complete and non-stubbed.

## Threat Surface

No new threat surface beyond the plan's documented threat model. T-02-23, T-02-24 mitigations are implemented:
- T-02-23: `AudioDispatcher::new()` validates each bundled WAV via `Decoder::try_from()` at construction.
- T-02-24: `_stream: MixerDeviceSink` held in struct for process lifetime.

## Issues Encountered

None beyond the auto-fixed API deviations above.

## User Setup Required

None — audio device is optional (failure returns `Err`, popup continues silently).

## Next Phase Readiness

- `audio::AudioDispatcher` + `audio::Tier` are ready for Plan 05's `popup_queue` to call `audio.play(tier)` once per popup fire
- All 3 unit tests pass without an audio device — safe for CI
- Plan 07 integration tests will exercise `AudioDispatcher::new()` + `play()` on real hardware

---
*Phase: 02-premium-ui-popup-companion-game-session*
*Completed: 2026-05-08*
