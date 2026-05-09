// scripts/gen_placeholder_sfx.rs
// Run: rustc scripts\gen_placeholder_sfx.rs -o gen_sfx.exe && .\gen_sfx.exe
// On Unix-like: rustc scripts/gen_placeholder_sfx.rs -o gen_sfx && ./gen_sfx
//
// v1 signature SFX generator. Three procedurally-synthesized WAV files:
//   popup-standard.wav  — D-05 workhorse ding,   ~480ms, -3 dBFS
//   popup-rare.wav      — D-06 richer sparkle,   ~750ms, -2 dBFS
//   popup-100pct.wav    — D-12 Do-Mi-Sol ascent, ~1200ms, -2 dBFS
//
// Format: WAV PCM 16-bit, 44100 Hz, mono — satisfies D-29 hard constraint and
// passes rodio::Decoder::try_from startup validation in src-tauri/src/audio.rs.
//
// DO NOT commit the compiled binary (gen_sfx.exe / gen_sfx). It is gitignored.
// Commit only this source file and the three WAV outputs.
use std::f32::consts::PI;
use std::fs::File;
use std::io::Write;

// ---------------------------------------------------------------------------
// WAV writer — PCM 16-bit, mono, little-endian
// ---------------------------------------------------------------------------
fn write_wav(path: &str, samples: &[i16], sr: u32) {
    let mut f = File::create(path).unwrap();
    let data_bytes = samples.len() * 2;
    let chunk_size = 36 + data_bytes;
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(chunk_size as u32).to_le_bytes()).unwrap();
    f.write_all(b"WAVE").unwrap();
    f.write_all(b"fmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();     // PCM chunk size
    f.write_all(&1u16.to_le_bytes()).unwrap();       // PCM format
    f.write_all(&1u16.to_le_bytes()).unwrap();       // mono
    f.write_all(&sr.to_le_bytes()).unwrap();
    f.write_all(&(sr * 2).to_le_bytes()).unwrap();   // byte rate (sr * channels * bits/8)
    f.write_all(&2u16.to_le_bytes()).unwrap();       // block align
    f.write_all(&16u16.to_le_bytes()).unwrap();      // bits per sample
    f.write_all(b"data").unwrap();
    f.write_all(&(data_bytes as u32).to_le_bytes()).unwrap();
    for s in samples {
        f.write_all(&s.to_le_bytes()).unwrap();
    }
}

// ---------------------------------------------------------------------------
// synth_segment — additive sine synthesis for one tonal segment
//
// Parameters:
//   duration_ms  — total segment length
//   sr           — sample rate (44100)
//   layers       — (frequency_hz, amplitude) pairs
//   attack_ms    — linear attack ramp duration
//   release_ms   — exponential release time constant (tau)
//
// Returns: f32 samples (not yet normalized or quantized).
// ---------------------------------------------------------------------------
fn synth_segment(
    duration_ms: u32,
    sr: u32,
    layers: &[(f32, f32)],
    attack_ms: f32,
    release_ms: f32,
) -> Vec<f32> {
    let n = (duration_ms as u64 * sr as u64 / 1000) as usize;
    let mut buf = vec![0.0f32; n];
    let attack_samples = (attack_ms / 1000.0 * sr as f32) as usize;
    // release_ms is the exponential time constant (tau), in ms
    let tau = release_ms / 1000.0;

    for &(freq, amp) in layers {
        for i in 0..n {
            let t = i as f32 / sr as f32;
            // Linear attack ramp
            let attack_env = if i < attack_samples {
                i as f32 / attack_samples.max(1) as f32
            } else {
                1.0
            };
            // Exponential release starting from t=0 (whole segment decays)
            let release_env = (-t / tau).exp();
            let env = attack_env * release_env;
            buf[i] += amp * env * (2.0 * PI * freq * t).sin();
        }
    }
    buf
}

// ---------------------------------------------------------------------------
// normalize_to_dbfs — scale f32 samples so peak reaches target_dbfs
// ---------------------------------------------------------------------------
fn normalize_to_dbfs(buf: &mut Vec<f32>, target_dbfs: f32) {
    let peak = buf.iter().map(|x| x.abs()).fold(0.0_f32, f32::max).max(1e-9);
    let target_linear = 10f32.powf(target_dbfs / 20.0);
    let scale = target_linear / peak;
    for x in buf.iter_mut() {
        *x *= scale;
    }
}

// ---------------------------------------------------------------------------
// to_i16 — clamp-quantize to 16-bit PCM
// ---------------------------------------------------------------------------
fn to_i16(buf: &[f32]) -> Vec<i16> {
    buf.iter()
        .map(|&x| (x * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect()
}

// ---------------------------------------------------------------------------
// Main — generate the three v1 signature SFX files
// ---------------------------------------------------------------------------
fn main() {
    // -----------------------------------------------------------------------
    // STANDARD — D-05 workhorse popup ding, ~480ms
    //
    // Layer composition:
    //   880 Hz  fundamental   @ 0.60 amp   — the "ding" body
    //   1760 Hz first harmonic @ 0.25 amp  — warmth and presence
    //   220 Hz  sub-octave    @ 0.15 amp   — low-end weight
    //
    // Envelope: 5ms linear attack, 400ms exponential release tau
    // Target: -3 dBFS peak
    // -----------------------------------------------------------------------
    let mut std_buf = synth_segment(
        480,
        44100,
        &[(880.0, 0.60), (1760.0, 0.25), (220.0, 0.15)],
        5.0,   // attack_ms
        400.0, // release tau ms
    );
    normalize_to_dbfs(&mut std_buf, -3.0);
    write_wav(
        "assets/sfx/popup-standard.wav",
        &to_i16(&std_buf),
        44100,
    );

    // -----------------------------------------------------------------------
    // RARE — D-06 richer/brighter tier, ~750ms
    //
    // Layer composition:
    //   1200 Hz fundamental    @ 0.55 amp  — bell-like primary tone
    //   2400 Hz second harmonic @ 0.30 amp — brightness
    //    600 Hz sub            @ 0.20 amp  — body/warmth
    //   3600 Hz shimmer        @ 0.10 amp  — high sparkle overtone
    //
    // Envelope: 8ms linear attack (slightly softer than standard), 500ms release tau
    // Target: -2 dBFS peak
    // -----------------------------------------------------------------------
    let mut rare_buf = synth_segment(
        750,
        44100,
        &[(1200.0, 0.55), (2400.0, 0.30), (600.0, 0.20), (3600.0, 0.10)],
        8.0,   // attack_ms
        500.0, // release tau ms
    );
    normalize_to_dbfs(&mut rare_buf, -2.0);
    write_wav(
        "assets/sfx/popup-rare.wav",
        &to_i16(&rare_buf),
        44100,
    );

    // -----------------------------------------------------------------------
    // COMPLETION — D-12 celebratory ascending Do-Mi-Sol (C5-E5-G5), ~1200ms
    //
    // Three 400ms segments concatenated, each with:
    //   fundamental + octave + second octave harmonics
    //   5ms attack, 350ms exponential release tau
    //
    // Note 1 (0–400ms):   C5 = 523.25 Hz
    // Note 2 (400–800ms): E5 = 659.25 Hz
    // Note 3 (800–1200ms): G5 = 784.00 Hz
    //
    // Each segment is synthesized independently; segments are concatenated
    // (not summed) so each note has its own clean attack.
    // Global peak normalization to -2 dBFS applied after concatenation.
    // -----------------------------------------------------------------------
    let seg1 = synth_segment(
        400,
        44100,
        &[(523.25, 0.7), (1046.50, 0.35), (2093.00, 0.15)],
        5.0,
        350.0,
    );
    let seg2 = synth_segment(
        400,
        44100,
        &[(659.25, 0.7), (1318.50, 0.35), (2637.00, 0.15)],
        5.0,
        350.0,
    );
    let seg3 = synth_segment(
        400,
        44100,
        &[(784.00, 0.7), (1568.00, 0.35), (3136.00, 0.15)],
        5.0,
        350.0,
    );

    // Concatenate the three segments into one buffer
    let mut comp_buf = Vec::with_capacity(seg1.len() + seg2.len() + seg3.len());
    comp_buf.extend_from_slice(&seg1);
    comp_buf.extend_from_slice(&seg2);
    comp_buf.extend_from_slice(&seg3);

    normalize_to_dbfs(&mut comp_buf, -2.0);
    write_wav(
        "assets/sfx/popup-100pct.wav",
        &to_i16(&comp_buf),
        44100,
    );

    println!(
        "ok — generated assets/sfx/popup-standard.wav (~480ms, -3dBFS), \
         popup-rare.wav (~750ms, -2dBFS), popup-100pct.wav (~1200ms, -2dBFS)"
    );
}
