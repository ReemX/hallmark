// scripts/gen_placeholder_sfx.rs
// Run: rustc scripts/gen_placeholder_sfx.rs -o /tmp/gen_sfx && /tmp/gen_sfx
// On Windows: rustc scripts\gen_placeholder_sfx.rs -o gen_sfx.exe && .\gen_sfx.exe
//
// Generates placeholder WAV files for Phase 2 development unblocking.
// These satisfy CONTEXT.md D-05/D-06/D-12 timing + dBFS specs but are NOT
// the signature sound. Phase 4 polish replaces them with the real mix.
use std::f32::consts::PI;
use std::fs::File;
use std::io::Write;

fn write_wav(path: &str, samples: &[i16], sr: u32) {
    let mut f = File::create(path).unwrap();
    let data_bytes = samples.len() * 2;
    let chunk = 36 + data_bytes;
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(chunk as u32).to_le_bytes()).unwrap();
    f.write_all(b"WAVE").unwrap();
    f.write_all(b"fmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();      // PCM chunk size
    f.write_all(&1u16.to_le_bytes()).unwrap();        // PCM format
    f.write_all(&1u16.to_le_bytes()).unwrap();        // mono
    f.write_all(&sr.to_le_bytes()).unwrap();
    f.write_all(&(sr * 2).to_le_bytes()).unwrap();    // byte rate
    f.write_all(&2u16.to_le_bytes()).unwrap();        // block align
    f.write_all(&16u16.to_le_bytes()).unwrap();       // bits/sample
    f.write_all(b"data").unwrap();
    f.write_all(&(data_bytes as u32).to_le_bytes()).unwrap();
    for s in samples { f.write_all(&s.to_le_bytes()).unwrap(); }
}

fn synth(duration_ms: u32, sr: u32, layers: &[(f32, f32)], peak_dbfs: f32) -> Vec<i16> {
    let n = (duration_ms as u64 * sr as u64 / 1000) as usize;
    let mut buf = vec![0.0f32; n];
    for (freq, amp) in layers {
        for i in 0..n {
            let t = i as f32 / sr as f32;
            // exponential envelope: short attack 5ms, long release.
            let env = (1.0 - (-t / 0.005).exp()) * (-t / 0.4).exp();
            buf[i] += amp * env * (2.0 * PI * freq * t).sin();
        }
    }
    // normalize to peak_dbfs
    let peak = buf.iter().map(|x| x.abs()).fold(0.0_f32, f32::max).max(1e-9);
    let target = 10f32.powf(peak_dbfs / 20.0);
    let scale = target / peak;
    buf.iter().map(|x| (x * scale * 32767.0) as i16).collect()
}

fn main() {
    // D-05 Standard: ~900ms, peak -8dBFS, layered ding (1200Hz) + riser (440-880Hz)
    let standard = synth(900, 44100, &[(1200.0, 1.0), (660.0, 0.6), (220.0, 0.3)], -8.0);
    write_wav("assets/sfx/popup-standard.wav", &standard, 44100);

    // D-06 Rare: ~1100ms, peak -5dBFS, base + sparkle (2400Hz) + choir (330Hz)
    let rare = synth(1100, 44100, &[(1200.0, 1.0), (660.0, 0.6), (220.0, 0.3), (2400.0, 0.5), (330.0, 0.4)], -5.0);
    write_wav("assets/sfx/popup-rare.wav", &rare, 44100);

    // D-12 100% Celebration: ~1800ms, 4-layer mix
    let comp = synth(1800, 44100, &[(1200.0, 0.9), (660.0, 0.7), (440.0, 0.6), (3300.0, 0.4)], -4.0);
    write_wav("assets/sfx/popup-100pct.wav", &comp, 44100);
    println!("ok — generated assets/sfx/popup-standard.wav, popup-rare.wav, popup-100pct.wav");
}
