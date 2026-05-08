# SFX assets — Phase 2 placeholders

The three WAV files in this directory (`popup-standard.wav`, `popup-rare.wav`,
`popup-100pct.wav`) are SYNTHESIZED PLACEHOLDERS generated from
`scripts/gen_placeholder_sfx.rs` for Phase 2 development unblocking.

They satisfy CONTEXT.md D-05/D-06/D-12 timing + dBFS specifications but are
NOT the signature sound. Phase 4 polish or a dedicated audio-design pass
must replace them with the real signature mix before public release.

To regenerate: `rustc scripts/gen_placeholder_sfx.rs -o gen_sfx && ./gen_sfx`.

**Phase 4 polish TODO (W-9):** Before any public release, replace these synthetic
placeholders with the locked signature mix:
- `popup-standard.wav` — D-05 ding + subtle riser + whoosh, ~900ms, peak −8dBFS.
- `popup-rare.wav` — D-06 same base + sparkle/choir stab layer, ~1100ms, peak −5dBFS.
- `popup-100pct.wav` — D-12 4-stem celebration mix, extended.
Tracked under CONTEXT.md `## Deferred Ideas`. Until then, the placeholders satisfy
the rodio decode contract and let Phase 2 ship behavior end-to-end.
To replace with real audio: drop the new WAV files in this directory with
the same filenames; rodio 0.22 wav decoder + include_bytes! pick them up
on next `cargo build`.

Format (when replaced): WAV PCM 16-bit, 44.1kHz or 48kHz, mono or stereo,
<500KB per file.
