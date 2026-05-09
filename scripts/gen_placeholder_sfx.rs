// scripts/gen_placeholder_sfx.rs
// Run: rustc scripts\gen_placeholder_sfx.rs -o gen_sfx.exe && .\gen_sfx.exe
// On Unix-like: rustc scripts/gen_placeholder_sfx.rs -o gen_sfx && ./gen_sfx
//
// =============================================================================
// RETIRED FOR v1 — D-28 OPTION 2 (maintainer-supplied audio) was chosen.
// This generator is retained as a v1.1 fallback. The three WAV files currently
// shipped in assets/sfx/ are NOT produced by this script; see assets/sfx/README.md.
// =============================================================================
//
// v1.1 fallback SFX generator. Produces three procedurally-synthesized WAVs:
//   popup-standard.wav  — D-05 workhorse ding,   ~480ms, -3 dBFS
//   popup-rare.wav      — D-06 richer sparkle,   ~750ms, -2 dBFS
//   popup-100pct.wav    — D-12 Do-Mi-Sol ascent, ~1200ms, -2 dBFS
//
// Format: WAV PCM 16-bit, 44100 Hz, mono — satisfies D-29 hard constraint and
// passes rodio::Decoder::try_from startup validation in src-tauri/src/audio.rs.
//
// Synthesis approach (v3 — "warm marimba" after v2 inharmonic bell tested as harsh):
//
//   2-3 sine partials at integer ratios (clean tone, no inharmonicity)
// + 1 low-index FM operator (C:M = 1:1.4142, modulation_index ≈ 0.6) — woody
//   character without the metallic clang of high-index FM
//   → cosine attack (6-10 ms — softer than v2)
//   → exponential release with long tau (300-500 ms)
//   → 1-pole low-pass filter at ~3.5 kHz (kills harshness, "Windows alert"
//     edge gone)
//   → tanh soft saturation, drive ≈ 1.2 (very subtle)
//   → tiny Schroeder reverb (40-80 ms tail, 8-12% wet — hint of room only)
//   → DC blocker + cosine tail fade-out
//   → normalize to target dBFS
//   → quantize to i16
//
// Why this is different from v2:
//   v2 stacked Risset's 11 inharmonic partials + a strong-index FM at C:M=3.5.
//   Both add metallic clang. User feedback: "ear-rape of a metal pipe."
//   v3 strips back to: warm fundamentals + subtle woody FM + LP roll-off +
//   minimal reverb. Closer to iOS Tritone (recorded marimba) than to a church
//   bell. PS5 trophy is bell-like but crystalline — we choose marimba-like
//   because mathematical bells in pure code reliably read as "harsh".
//
// References:
//   Apple Tritone is a recorded marimba — https://www.20k.org/episodes/the-sound-of-apple
//   Chowning FM bell                     — https://ccrma.stanford.edu/software/clm/compmus/clm-tutorials/fm2.html
//   Schroeder reverb                     — https://ccrma.stanford.edu/~jos/pasp/Schroeder_Allpass_Sections.html
//
// DO NOT commit the compiled binary (gen_sfx.exe / gen_sfx). It is gitignored.
// Commit only this source file and the three WAV outputs.
use std::f32::consts::PI;
use std::fs::File;
use std::io::Write;

const SR: u32 = 44100;

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
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap();
    f.write_all(&sr.to_le_bytes()).unwrap();
    f.write_all(&(sr * 2).to_le_bytes()).unwrap();
    f.write_all(&2u16.to_le_bytes()).unwrap();
    f.write_all(&16u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&(data_bytes as u32).to_le_bytes()).unwrap();
    for s in samples {
        f.write_all(&s.to_le_bytes()).unwrap();
    }
}

// ---------------------------------------------------------------------------
// Additive sine partials at integer/clean ratios. Each partial gets its own
// amplitude and decay tau (high partials die faster than low partials).
// Slight (≤5 cent) random-walk detune flag adds a touch of beating warmth.
// ---------------------------------------------------------------------------
fn additive_partials(
    f0: f32,
    partials: &[(f32, f32, f32)], // (freq_mult, amp, tau_ms)
    detune_cents: f32,            // small detune on alternating partials
    dur_ms: u32,
    sr: u32,
) -> Vec<f32> {
    let n = (dur_ms as u64 * sr as u64 / 1000) as usize;
    let mut out = vec![0.0_f32; n];
    let dt = 1.0 / sr as f32;
    for (k, &(fr, ar, tau_ms)) in partials.iter().enumerate() {
        // Apply detune alternating (+ / -) per partial — produces slow beats
        let sign: f32 = if k % 2 == 0 { 1.0 } else { -1.0 };
        let r = 2f32.powf(sign * detune_cents / 1200.0);
        let f = f0 * fr * r;
        let tau_s = tau_ms / 1000.0;
        let decay_coef = (-1.0_f32 / (tau_s * sr as f32)).exp();
        let mut env = ar;
        let two_pi_f = 2.0 * PI * f;
        for i in 0..n {
            let t = i as f32 * dt;
            out[i] += env * (two_pi_f * t).sin();
            env *= decay_coef;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// FM operator — woody mallet character only. Keep modulation index LOW
// (≈0.4–1.0) and C:M ratio at ≈1.4142 to stay marimba-side, NOT bell-side.
// Higher idx + ratio=3.5 made v2 sound metallic; we deliberately avoid it.
// ---------------------------------------------------------------------------
fn fm_voice_woody(
    fc: f32,
    cm_ratio: f32,
    idx_start: f32,
    idx_tau_ms: f32,
    dur_ms: u32,
    sr: u32,
) -> Vec<f32> {
    let n = (dur_ms as u64 * sr as u64 / 1000) as usize;
    let mut out = vec![0.0_f32; n];
    let fm = fc * cm_ratio;
    let tau_s = idx_tau_ms / 1000.0;
    let idx_decay = (-1.0_f32 / (tau_s * sr as f32)).exp();
    let mut idx = idx_start;
    let two_pi_fc = 2.0 * PI * fc;
    let two_pi_fm = 2.0 * PI * fm;
    let dt = 1.0 / sr as f32;
    for i in 0..n {
        let t = i as f32 * dt;
        let mod_sig = idx * (two_pi_fm * t).sin();
        out[i] = (two_pi_fc * t + mod_sig).sin();
        idx *= idx_decay;
    }
    out
}

// ---------------------------------------------------------------------------
// 1-pole low-pass filter (RC-style). Critical for premium feel — strips the
// metallic top-end glare that math-synthesis produces. Cutoff ~3.5 kHz keeps
// the body intact while killing harshness above the 2nd-3rd harmonic.
// ---------------------------------------------------------------------------
fn lowpass_1pole(buf: &mut [f32], cutoff_hz: f32, sr: u32) {
    // y[n] = α·x[n] + (1-α)·y[n-1]
    let rc = 1.0 / (2.0 * PI * cutoff_hz);
    let dt = 1.0 / sr as f32;
    let alpha = dt / (rc + dt);
    let mut y = 0.0_f32;
    for x in buf.iter_mut() {
        y = alpha * *x + (1.0 - alpha) * y;
        *x = y;
    }
}

// ---------------------------------------------------------------------------
// Cosine attack (S-curve, no derivative discontinuity)
// ---------------------------------------------------------------------------
fn cosine_attack(buf: &mut [f32], attack_ms: f32, sr: u32) {
    let attack_n = (attack_ms / 1000.0 * sr as f32) as usize;
    let n_take = attack_n.min(buf.len());
    for i in 0..n_take {
        let p = i as f32 / attack_n.max(1) as f32;
        let env = 0.5 - 0.5 * (PI * p).cos();
        buf[i] *= env;
    }
}

// ---------------------------------------------------------------------------
// Mix src into target additively
// ---------------------------------------------------------------------------
fn mix_into(target: &mut Vec<f32>, src: &[f32], gain: f32) {
    if target.len() < src.len() {
        target.resize(src.len(), 0.0);
    }
    for (i, &s) in src.iter().enumerate() {
        target[i] += s * gain;
    }
}

// ---------------------------------------------------------------------------
// Soft saturation via tanh — keep drive subtle (1.1-1.3) for warmth without
// adding harmonics that LP filter can't catch.
// ---------------------------------------------------------------------------
fn soft_clip(buf: &mut [f32], drive: f32) {
    let knee = drive.tanh();
    for x in buf.iter_mut() {
        *x = (*x * drive).tanh() / knee;
    }
}

// ---------------------------------------------------------------------------
// Schroeder reverb — keep feedback LOW (0.5-0.7) and wet mix LOW (5-12%)
// for "hint of room" rather than "cathedral".
// ---------------------------------------------------------------------------
struct Comb {
    buf: Vec<f32>,
    idx: usize,
    fb: f32,
}
impl Comb {
    fn new(d: usize, fb: f32) -> Self {
        Self { buf: vec![0.0; d], idx: 0, fb }
    }
    fn process(&mut self, x: f32) -> f32 {
        let y = self.buf[self.idx];
        self.buf[self.idx] = x + y * self.fb;
        self.idx = (self.idx + 1) % self.buf.len();
        y
    }
}
struct Allpass {
    buf: Vec<f32>,
    idx: usize,
    g: f32,
}
impl Allpass {
    fn new(d: usize, g: f32) -> Self {
        Self { buf: vec![0.0; d], idx: 0, g }
    }
    fn process(&mut self, x: f32) -> f32 {
        let b = self.buf[self.idx];
        let y = -self.g * x + b;
        self.buf[self.idx] = x + self.g * b;
        self.idx = (self.idx + 1) % self.buf.len();
        y
    }
}

fn apply_reverb(dry: &[f32], wet_mix: f32, fb: f32, sr: u32, tail_ms: u32) -> Vec<f32> {
    let mut combs = [
        Comb::new(1557, fb),
        Comb::new(1617, fb),
        Comb::new(1491, fb),
        Comb::new(1422, fb),
    ];
    let mut aps = [Allpass::new(225, 0.5), Allpass::new(556, 0.5)];
    let tail_n = (tail_ms as u64 * sr as u64 / 1000) as usize;
    let total_n = dry.len() + tail_n;
    let mut out = Vec::with_capacity(total_n);
    for i in 0..total_n {
        let x = if i < dry.len() { dry[i] } else { 0.0 };
        let mut w = 0.0;
        for c in combs.iter_mut() {
            w += c.process(x);
        }
        w *= 0.25;
        for a in aps.iter_mut() {
            w = a.process(w);
        }
        out.push(x * (1.0 - wet_mix) + w * wet_mix);
    }
    out
}

// ---------------------------------------------------------------------------
// DC blocker
// ---------------------------------------------------------------------------
fn dc_block(buf: &mut [f32]) {
    let mut x_prev = 0.0_f32;
    let mut y_prev = 0.0_f32;
    for x in buf.iter_mut() {
        let y = *x - x_prev + 0.995 * y_prev;
        x_prev = *x;
        *x = y;
        y_prev = y;
    }
}

// ---------------------------------------------------------------------------
// Cosine tail fade-out — guarantees zero last sample
// ---------------------------------------------------------------------------
fn cosine_tail_fade(buf: &mut [f32], tail_ms: f32, sr: u32) {
    let tail_n = (tail_ms / 1000.0 * sr as f32) as usize;
    let n = buf.len();
    if tail_n == 0 || tail_n > n {
        return;
    }
    for i in 0..tail_n {
        let p = i as f32 / tail_n as f32;
        let env = 0.5 + 0.5 * (PI * p).cos();
        buf[n - tail_n + i] *= env;
    }
}

fn normalize_to_dbfs(buf: &mut [f32], target_dbfs: f32) {
    let peak = buf.iter().map(|x| x.abs()).fold(0.0_f32, f32::max).max(1e-9);
    let target_linear = 10f32.powf(target_dbfs / 20.0);
    let scale = target_linear / peak;
    for x in buf.iter_mut() {
        *x *= scale;
    }
}

fn to_i16(buf: &[f32]) -> Vec<i16> {
    buf.iter()
        .map(|&x| (x * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect()
}

// ---------------------------------------------------------------------------
// Build a single warm marimba-style voice. Returns the raw mixed signal
// before saturation/reverb.
//
// f0 is the fundamental. The voice consists of:
//   - additive sines: fundamental + octave + sub at decay-weighted amps
//   - 1 woody FM operator (low idx) for mallet character
// ---------------------------------------------------------------------------
fn build_warm_voice(
    f0: f32,
    body_ms: u32,
    additive_gain: f32,
    fm_gain: f32,
    sr: u32,
) -> Vec<f32> {
    // Additive partials — fundamental + octave + sub-octave, with subtle detune
    let partials = &[
        (1.0, 0.85,  500.0),  // fundamental — long ring
        (2.0, 0.30,  220.0),  // octave — medium decay
        (0.5, 0.20,  600.0),  // sub-octave — body / weight
    ];
    let mut additive = additive_partials(f0, partials, 3.0, body_ms, sr);

    // FM woody mallet — LOW modulation index, woody C:M ratio
    let mut fm = fm_voice_woody(f0, 1.4142, 0.6, 90.0, body_ms, sr);

    let mut mix = vec![0.0_f32; (body_ms as usize * sr as usize) / 1000];
    mix_into(&mut mix, &additive, additive_gain);
    mix_into(&mut mix, &fm, fm_gain);

    // Suppress unused-warning by re-binding (rust drops them anyway)
    let _ = (additive.len(), fm.len());
    additive.clear();
    fm.clear();
    mix
}

// ---------------------------------------------------------------------------
// Apply the post pipeline: cosine attack → LP filter → soft clip → reverb
//   → DC block → tail fade → normalize → i16
// ---------------------------------------------------------------------------
fn finalize(
    mut mix: Vec<f32>,
    attack_ms: f32,
    lp_cutoff_hz: f32,
    drive: f32,
    reverb_wet: f32,
    reverb_fb: f32,
    reverb_tail_ms: u32,
    fade_ms: f32,
    target_dbfs: f32,
    sr: u32,
) -> Vec<i16> {
    cosine_attack(&mut mix, attack_ms, sr);
    lowpass_1pole(&mut mix, lp_cutoff_hz, sr);
    soft_clip(&mut mix, drive);
    let mut wet = apply_reverb(&mix, reverb_wet, reverb_fb, sr, reverb_tail_ms);
    dc_block(&mut wet);
    cosine_tail_fade(&mut wet, fade_ms, sr);
    normalize_to_dbfs(&mut wet, target_dbfs);
    to_i16(&wet)
}

// ---------------------------------------------------------------------------
// Main — generate the three v1 signature SFX files.
// ---------------------------------------------------------------------------
fn main() {
    // -----------------------------------------------------------------------
    // STANDARD — D-05 workhorse popup ding, ~480ms (with 60ms reverb tail).
    //   f0 = 660 Hz (E5) — lower than v2's 880 Hz to reduce piercing top
    //   Additive @ 1.0 + woody FM @ 0.35
    //   LP @ 3500 Hz, drive=1.15 (very subtle), reverb 60ms / 8% wet
    // -----------------------------------------------------------------------
    {
        let body_ms = 420;
        let mix = build_warm_voice(660.0, body_ms, 1.0, 0.35, SR);
        let pcm = finalize(
            mix,
            8.0,    // attack
            3500.0, // LP cutoff
            1.15,   // saturation drive (very subtle)
            0.08,   // reverb wet
            0.55,   // reverb feedback
            60,     // reverb tail ms
            10.0,   // tail fade ms
            -3.0,   // target dBFS
            SR,
        );
        write_wav("assets/sfx/popup-standard.wav", &pcm, SR);
    }

    // -----------------------------------------------------------------------
    // RARE — D-06 richer/brighter tier, ~750ms.
    //   f0 = 880 Hz (A5) — slightly brighter than standard, less piercing
    //   Additive @ 1.0 + woody FM @ 0.5 (more mallet character)
    //   LP @ 4500 Hz (more brilliance), drive=1.25, reverb 100ms / 14% wet
    // -----------------------------------------------------------------------
    {
        let body_ms = 650;
        let mix = build_warm_voice(880.0, body_ms, 1.0, 0.5, SR);
        let pcm = finalize(
            mix,
            10.0,   // attack
            4500.0, // LP cutoff (more highs through)
            1.25,   // saturation drive
            0.14,   // reverb wet
            0.65,   // reverb feedback
            100,    // reverb tail ms
            10.0,   // tail fade ms
            -2.0,   // target dBFS
            SR,
        );
        write_wav("assets/sfx/popup-rare.wav", &pcm, SR);
    }

    // -----------------------------------------------------------------------
    // COMPLETION — D-12 celebratory ascending Do-Mi-Sol (C5-E5-G5), ~1200ms.
    //   Three notes overlapping (each 380ms, stride 320ms).
    //   Each note uses build_warm_voice; final note slightly longer.
    //   LP @ 4000 Hz, drive=1.2, reverb 150ms / 18% wet (a hint more "room").
    // -----------------------------------------------------------------------
    {
        let note_ms = 380u32;
        let stride_ms = 320u32;
        let stride_n = stride_ms as usize * SR as usize / 1000;
        let n1 = build_warm_voice(523.25, note_ms, 1.0, 0.45, SR);
        let n2 = build_warm_voice(659.25, note_ms, 1.0, 0.45, SR);
        let n3 = build_warm_voice(784.0, note_ms + 100, 1.0, 0.55, SR); // hold last note slightly
        let total_n = stride_n * 2 + n3.len();
        let mut comp = vec![0.0_f32; total_n];
        for (i, &s) in n1.iter().enumerate() { comp[i] += s; }
        for (i, &s) in n2.iter().enumerate() { comp[stride_n + i] += s; }
        for (i, &s) in n3.iter().enumerate() { comp[stride_n * 2 + i] += s; }

        let pcm = finalize(
            comp,
            8.0,    // attack
            4000.0, // LP cutoff
            1.20,   // saturation drive
            0.18,   // reverb wet
            0.70,   // reverb feedback
            150,    // reverb tail ms
            12.0,   // tail fade ms
            -2.0,   // target dBFS
            SR,
        );
        write_wav("assets/sfx/popup-100pct.wav", &pcm, SR);
    }

    println!(
        "ok — generated assets/sfx/popup-standard.wav (~480ms, -3dBFS), \
         popup-rare.wav (~750ms, -2dBFS), popup-100pct.wav (~1200ms, -2dBFS) \
         using warm-marimba pipeline (additive + low-idx FM + LP@3.5-4.5kHz + tiny reverb)"
    );
}
