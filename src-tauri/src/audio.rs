//! rodio-based AudioDispatcher. Plays the three signature-style SFX variants
//! (Standard / Rare / Completion) from pre-decoded bundled WAV bytes.
//!
//! IMPORTANT (RESEARCH.md Pitfall 1): rodio 0.22 renamed `OutputStream` →
//! `MixerDeviceSink` and `Sink` → `Player`. CLAUDE.md cites the 0.20-style
//! API; this module uses 0.22.
//!
//! IMPORTANT (RESEARCH.md anti-pattern): the `MixerDeviceSink` MUST be held
//! in the AudioDispatcher struct for the process lifetime — dropping it
//! silences all audio.

use std::io::Cursor;
use std::sync::Arc;

use rodio::mixer::Mixer;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink};

/// Tier — selects which SFX to play. Plan 05's popup_queue maps an unlocked
/// achievement's classification to a Tier and calls AudioDispatcher::play(tier).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Standard,    // D-05 — default popup
    Rare,        // D-06 — global_pct < 10% with rarity available
    Completion,  // D-12 — 100% achievement set unlocked, once per app_id
}

/// Pre-loaded SFX dispatcher. Construct once at process start; call play() per popup.
pub struct AudioDispatcher {
    // Held for process lifetime — dropping silences all output.
    _stream: MixerDeviceSink,
    mixer: Mixer,
    standard_bytes: Arc<Vec<u8>>,
    rare_bytes: Arc<Vec<u8>>,
    completion_bytes: Arc<Vec<u8>>,
}

impl AudioDispatcher {
    /// Open the default audio output device and pre-load the three bundled
    /// SFX assets. Returns Err if no default device is available — caller
    /// (Plan 07's setup) logs the warning and continues with silent popups.
    pub fn new() -> anyhow::Result<Self> {
        let stream = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| anyhow::anyhow!("rodio open_default_sink failed: {e}"))?;
        let mixer = stream.mixer().clone();

        // Bundle SFX in the binary at compile time. Phase 4 may switch to
        // filesystem reads from the install dir for user-replaceable signatures.
        let standard_bytes = Arc::new(
            include_bytes!("../../assets/sfx/popup-standard.wav").to_vec()
        );
        let rare_bytes = Arc::new(
            include_bytes!("../../assets/sfx/popup-rare.wav").to_vec()
        );
        let completion_bytes = Arc::new(
            include_bytes!("../../assets/sfx/popup-100pct.wav").to_vec()
        );

        // Validate that each bundled WAV decodes — catches corrupt assets
        // at process start rather than first popup.
        for (name, bytes) in [
            ("standard", &standard_bytes),
            ("rare", &rare_bytes),
            ("completion", &completion_bytes),
        ] {
            let cursor = Cursor::new(bytes.as_ref().clone());
            if let Err(e) = Decoder::try_from(cursor) {
                anyhow::bail!("bundled SFX '{name}.wav' failed to decode: {e}");
            }
        }

        tracing::info!("AudioDispatcher initialized (3 SFX bundles validated)");
        Ok(Self { _stream: stream, mixer, standard_bytes, rare_bytes, completion_bytes })
    }

    /// Non-blocking. Decodes from in-memory bytes (cheap) and pushes to mixer.
    /// Concurrent calls layer in the mixer (rare-tier doesn't get clipped by
    /// a still-tailing standard from the previous popup).
    ///
    /// Returns Err on decode failure or mixer-add failure; popup_queue logs
    /// at warn and continues — visual popup still fires.
    pub fn play(&self, tier: Tier) -> anyhow::Result<()> {
        let bytes = match tier {
            Tier::Standard => self.standard_bytes.clone(),
            Tier::Rare => self.rare_bytes.clone(),
            Tier::Completion => self.completion_bytes.clone(),
        };
        let cursor = Cursor::new(bytes.as_ref().clone());
        let decoder = Decoder::try_from(cursor)
            .map_err(|e| anyhow::anyhow!("decode failed for tier {:?}: {}", tier, e))?;
        // mixer.add takes a Source; layered/concurrent (vs Player::append which
        // is strictly sequential). The mixer auto-converts sample rate + channels.
        // RESEARCH.md Section D rationale.
        self.mixer.add(decoder);
        tracing::debug!(?tier, "audio play dispatched");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On CI without an audio device, AudioDispatcher::new() will fail.
    /// Test the partial-construction path: verify the bundled SFX bytes
    /// are non-empty + start with RIFF magic. This catches "forgot to commit
    /// the WAV files" without requiring an audio device at test time.
    #[test]
    fn bundled_sfx_bytes_have_riff_magic() {
        let standard = include_bytes!("../../assets/sfx/popup-standard.wav");
        let rare = include_bytes!("../../assets/sfx/popup-rare.wav");
        let completion = include_bytes!("../../assets/sfx/popup-100pct.wav");

        for (name, b) in [("standard", &standard[..]), ("rare", &rare[..]), ("completion", &completion[..])] {
            assert!(b.len() > 100, "{name} too small: {}", b.len());
            assert_eq!(&b[0..4], b"RIFF", "{name} missing RIFF magic");
            assert_eq!(&b[8..12], b"WAVE", "{name} missing WAVE magic");
        }
    }

    /// Test the decoder path WITHOUT touching the audio device. We construct
    /// a Decoder from the bundled bytes and confirm it parses; `mixer.add`
    /// itself we cannot test without a device.
    #[test]
    fn bundled_sfx_decode_via_rodio() {
        for path_label in ["standard", "rare", "completion"] {
            let bytes: &[u8] = match path_label {
                "standard" => include_bytes!("../../assets/sfx/popup-standard.wav"),
                "rare" => include_bytes!("../../assets/sfx/popup-rare.wav"),
                "completion" => include_bytes!("../../assets/sfx/popup-100pct.wav"),
                _ => unreachable!(),
            };
            let cursor = Cursor::new(bytes.to_vec());
            let decoder = Decoder::try_from(cursor);
            assert!(decoder.is_ok(), "{path_label}.wav failed to decode: {:?}", decoder.err());
        }
    }

    #[test]
    fn tier_enum_variants_distinct() {
        assert_ne!(Tier::Standard as u8, Tier::Rare as u8);
        assert_ne!(Tier::Rare as u8, Tier::Completion as u8);
        assert_ne!(Tier::Standard as u8, Tier::Completion as u8);
    }

    // The `new()` constructor and `play()` are exercised end-to-end in Plan 07's
    // integration tests, which run on a host with a real audio device. We do
    // NOT add a constructor test here because cargo test on CI typically lacks
    // a default audio device (and gracefully erroring is the documented behavior).
}
