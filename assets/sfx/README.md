# SFX assets — v1 signature audio

The three WAV files in this directory ship as the v1 Hallmark popup SFX:

| File | Tier | Format | Size |
|------|------|--------|------|
| `popup-standard.wav` | Standard (D-05) | PCM 16-bit, 48 kHz, stereo | ~103 KB |
| `popup-rare.wav` | Rare (D-06) | PCM 16-bit, 48 kHz, stereo | ~214 KB |
| `popup-100pct.wav` | 100% completion (D-12) | PCM 16-bit, 48 kHz, stereo | ~214 KB |

Format envelope satisfies D-29 (PCM 16-bit, 44.1 kHz or 48 kHz, mono or
stereo) and `audio.rs::AudioDispatcher::new()` startup decode validation
(see `src-tauri/src/audio.rs` lines 60–69 — Pitfall 9 protection).

## Provenance (D-28)

Source: **maintainer-supplied** (option 2 path of D-28). The two procedural
synthesis attempts (Risset's inharmonic bell + dual FM, then warm-marimba +
LP filter) audited as too harsh in v1 development. RESEARCH § Open Questions
\#1 explicitly endorsed this fallback: *"DO NOT block Phase 4 release on
subjective audio quality."*

`popup-100pct.wav` is currently a copy of `popup-rare.wav` — the dedicated
100% celebration mix is deferred to **v1.1**. The popup queue's idle-50 ms
celebration ordering (D-12) still works correctly with the rare-tier sound
in the celebration slot; users hear a louder/longer rare ding when they hit
100% rather than a distinct fanfare.

License: **unspecified for v1**. Treat as "all rights reserved by the
project maintainer" until a clean license declaration lands in v1.1.
Forks redistributing the bundle should swap these assets for known-CC0
or self-authored alternatives.

## Replacing the SFX

Drop replacement WAV files at the same paths with the same filenames.
`audio.rs` uses `include_bytes!` so a `cargo build` rebake is required for
the changes to ship. The dispatcher decodes each file at startup and
panics with `bundled SFX 'X.wav' failed to decode` if any file is malformed
(PCM-only, no MP3/Vorbis/Opus inside the WAV container — D-29).

## Procedural fallback (v1.1 reserve)

`scripts/gen_placeholder_sfx.rs` retains the v3 "warm marimba" synthesis
pipeline (additive partials + low-index FM + LP filter + Schroeder reverb)
as a v1.1 fallback in case the maintainer-supplied audio needs to be
swapped without re-curating new files. Regenerate with:

```sh
rustc scripts/gen_placeholder_sfx.rs -o gen_sfx.exe && ./gen_sfx.exe
```

The generator overwrites the three WAV paths above. Do NOT commit `gen_sfx.exe`.
