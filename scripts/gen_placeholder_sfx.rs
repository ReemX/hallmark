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
// Synthesis pipeline (per tier):
//   risset_bell (11 inharmonic partials, per-partial decay)
// + fm_voice  (C:M = 1:1.4142, exp-decaying modulation index)
// + fm_voice  (C:M = 1:3.5, low-amp shimmer, fast modulator decay)  [rare/comp]
//   → unison detune ±4 cents on FM voices
//   → cosine attack (4–6 ms)
//   → tanh soft saturation (drive 1.6–1.8)
//   → Schroeder reverb (4 combs + 2 allpasses, 80–200 ms tail)
//   → DC blocker (1-pole HPF)
//   → cosine tail fade-out (8 ms, guarantees zero-crossing end)
//   → normalize to target dBFS
//   → quantize to i16
//
// References:
//   Risset's 11-partial bell — http://msp.ucsd.edu/techniques/v0.11/book-html/node71.html
//   Chowning FM bell        — https://ccrma.stanford.edu/software/clm/compmus/clm-tutorials/fm2.html
//   Schroeder reverb        — https://ccrma.stanford.edu/~jos/pasp/Schroeder_Allpass_Sections.html
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
// Risset's canonical 11-partial bell — inharmonic ratios with per-partial
// decay times. This single function moves output from "Windows alert" to
// "premium bell" more than any other change. The 0.56/0.563 and 0.92/0.923
// pairs are deliberate beating partials (~3 Hz beat) — that's the shimmer.
// ---------------------------------------------------------------------------
const RISSET_PARTIALS: &[(f32, f32, f32)] = &[
    // (freq_mult, amp_mult, tau_ms)
    (0.56,  1.00, 1000.0),
    (0.563, 0.67, 1000.0),
    (0.92,  1.00,  700.0),
    (0.923, 1.80,  700.0),
    (1.19,  2.67,  400.0),
    (1.70,  1.46,  300.0),
    (2.00,  1.33,  200.0),
    (2.74,  1.33,  150.0),
    (3.00,  1.00,  150.0),
    (3.74,  1.33,  100.0),
    (4.07,  0.75,   80.0),
];

fn risset_bell(f0: f32, dur_ms: u32, sr: u32) -> Vec<f32> {
    let n = (dur_ms as u64 * sr as u64 / 1000) as usize;
    let mut out = vec![0.0_f32; n];
    for &(fr, ar, tau_ms) in RISSET_PARTIALS {
        let f = f0 * fr;
        let tau_s = tau_ms / 1000.0;
        let decay = (-1.0_f32 / (tau_s * sr as f32)).exp();
        let mut env = ar;
        let two_pi_f = 2.0 * PI * f;
        let dt = 1.0 / sr as f32;
        for i in 0..n {
            let t = i as f32 * dt;
            out[i] += env * (two_pi_f * t).sin();
            env *= decay;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// FM operator — y = sin(2π·fc·t + I(t)·sin(2π·fm·t))
// I(t) decays exponentially from idx_start to ~0 with tau idx_tau_ms.
// Use C:M ratios:
//   1.4142 (≈ √2) → woody mallet character
//   3.5            → classic DX7 TUB BELLS metallic shimmer
// ---------------------------------------------------------------------------
fn fm_voice(
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
// Slow amplitude envelope on top of the per-partial decay — applies a global
// exp-decay shape with cosine attack to taper the whole voice.
// ---------------------------------------------------------------------------
fn apply_global_envelope(buf: &mut [f32], attack_ms: f32, tau_ms: f32, sr: u32) {
    let n = buf.len();
    let attack_n = (attack_ms / 1000.0 * sr as f32) as usize;
    let tau_s = tau_ms / 1000.0;
    let decay_coef = (-1.0_f32 / (tau_s * sr as f32)).exp();
    let mut env = 1.0_f32;
    for i in 0..n {
        // Cosine attack S-curve (smoother than linear, no derivative discontinuity)
        let attack_env = if i < attack_n {
            let p = i as f32 / attack_n.max(1) as f32;
            0.5 - 0.5 * (PI * p).cos()
        } else {
            1.0
        };
        buf[i] *= attack_env * env;
        env *= decay_coef;
    }
}

// ---------------------------------------------------------------------------
// Sum a buffer into a target buffer at given gain — additive mix helper.
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
// Unison detune helper — generate three copies of an FM voice slightly
// detuned (±cents) and average. Adds beating thickness.
// ---------------------------------------------------------------------------
fn fm_voice_detuned(
    fc: f32,
    cm_ratio: f32,
    idx_start: f32,
    idx_tau_ms: f32,
    dur_ms: u32,
    sr: u32,
    cents: f32,
) -> Vec<f32> {
    let r = 2f32.powf(cents / 1200.0);
    let a = fm_voice(fc, cm_ratio, idx_start, idx_tau_ms, dur_ms, sr);
    let b = fm_voice(fc * r, cm_ratio, idx_start, idx_tau_ms, dur_ms, sr);
    let c = fm_voice(fc / r, cm_ratio, idx_start, idx_tau_ms, dur_ms, sr);
    a.iter().zip(b.iter()).zip(c.iter())
        .map(|((x, y), z)| (x + y + z) / 3.0)
        .collect()
}

// ---------------------------------------------------------------------------
// Soft saturation via tanh — adds odd-harmonic warmth, glues unison voices,
// limits transients without hard clipping. Apply BEFORE normalization.
// ---------------------------------------------------------------------------
fn soft_clip(buf: &mut [f32], drive: f32) {
    let knee = drive.tanh();
    for x in buf.iter_mut() {
        *x = (*x * drive).tanh() / knee;
    }
}

// ---------------------------------------------------------------------------
// Schroeder reverb — 4 parallel feedback combs into 2 series allpasses.
// Delays in samples are coprime to avoid resonance pile-up. Mono in/out.
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

// Apply Schroeder reverb to dry signal. Extends the buffer by `tail_ms` to
// allow the reverb tail to ring out beyond the dry signal.
fn apply_reverb(dry: &[f32], wet_mix: f32, fb: f32, sr: u32, tail_ms: u32) -> Vec<f32> {
    // Coprime delay lengths (Freeverb-style proportions)
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
// DC blocker — 1-pole high-pass filter, removes asymmetric DC offset
// introduced by saturation. Run AFTER reverb, BEFORE final normalize.
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
// Cosine tail fade-out — guarantees the last sample is exactly 0.0,
// preventing end-of-file click on playback or retrigger.
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

// ---------------------------------------------------------------------------
// Normalize to target dBFS (peak normalize)
// ---------------------------------------------------------------------------
fn normalize_to_dbfs(buf: &mut [f32], target_dbfs: f32) {
    let peak = buf.iter().map(|x| x.abs()).fold(0.0_f32, f32::max).max(1e-9);
    let target_linear = 10f32.powf(target_dbfs / 20.0);
    let scale = target_linear / peak;
    for x in buf.iter_mut() {
        *x *= scale;
    }
}

// ---------------------------------------------------------------------------
// Clamp-quantize f32 → i16 PCM
// ---------------------------------------------------------------------------
fn to_i16(buf: &[f32]) -> Vec<i16> {
    buf.iter()
        .map(|&x| (x * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect()
}

// ---------------------------------------------------------------------------
// Build one tier voice: Risset bell + FM mallet + (optional) FM shimmer.
// Returns the raw mixed signal BEFORE post-processing (saturation, reverb).
// ---------------------------------------------------------------------------
fn build_tier_voice(
    f0: f32,
    body_ms: u32,
    risset_gain: f32,
    fm_woody_gain: f32,
    fm_metallic_gain: f32,
    sr: u32,
) -> Vec<f32> {
    // Risset 11-partial inharmonic bell — the perceptual core.
    let mut bell = risset_bell(f0, body_ms, sr);
    // Apply per-partial decays inside risset_bell already; add a soft global env
    // to taper the strike onset.
    apply_global_envelope(&mut bell, 4.0, 1000.0, sr);

    // FM operator at C:M = 1:1.4142 — woody mallet attack character.
    let mut fm_woody = fm_voice_detuned(f0, 1.4142, 4.0, 80.0, body_ms, sr, 4.0);
    apply_global_envelope(&mut fm_woody, 4.0, 200.0, sr);

    // FM operator at C:M = 1:3.5 — metallic DX7 bell shimmer (low amp).
    let mut fm_metallic = fm_voice(f0, 3.5, 5.0, 60.0, body_ms, sr);
    apply_global_envelope(&mut fm_metallic, 4.0, 120.0, sr);

    // Mix
    let n = body_ms as usize * sr as usize / 1000;
    let mut mix = vec![0.0_f32; n];
    mix_into(&mut mix, &bell, risset_gain);
    mix_into(&mut mix, &fm_woody, fm_woody_gain);
    mix_into(&mut mix, &fm_metallic, fm_metallic_gain);
    mix
}

// ---------------------------------------------------------------------------
// Main — generate the three v1 signature SFX files.
// ---------------------------------------------------------------------------
fn main() {
    // -----------------------------------------------------------------------
    // STANDARD — D-05 workhorse popup ding, ~480ms total (with 80ms reverb tail).
    //   f0 = 880 Hz (A5)
    //   Risset bell @ 1.0 + woody FM @ 0.6 + metallic FM @ 0.15
    //   Reverb: 80ms tail, 18% wet, fb=0.78
    // -----------------------------------------------------------------------
    {
        let body_ms = 400;
        let mut mix = build_tier_voice(880.0, body_ms, 1.0, 0.6, 0.15, SR);
        soft_clip(&mut mix, 1.6);
        let mut wet = apply_reverb(&mix, 0.18, 0.78, SR, 80);
        dc_block(&mut wet);
        cosine_tail_fade(&mut wet, 8.0, SR);
        normalize_to_dbfs(&mut wet, -3.0);
        write_wav("assets/sfx/popup-standard.wav", &to_i16(&wet), SR);
    }

    // -----------------------------------------------------------------------
    // RARE — D-06 richer/brighter tier, ~750ms.
    //   f0 = 1200 Hz
    //   Risset bell @ 1.0 + woody FM @ 0.55 + metallic FM @ 0.30 (more shimmer)
    //   Reverb: 150ms tail, 22% wet, fb=0.82
    // -----------------------------------------------------------------------
    {
        let body_ms = 600;
        let mut mix = build_tier_voice(1200.0, body_ms, 1.0, 0.55, 0.30, SR);
        soft_clip(&mut mix, 1.8);
        let mut wet = apply_reverb(&mix, 0.22, 0.82, SR, 150);
        dc_block(&mut wet);
        cosine_tail_fade(&mut wet, 10.0, SR);
        normalize_to_dbfs(&mut wet, -2.0);
        write_wav("assets/sfx/popup-rare.wav", &to_i16(&wet), SR);
    }

    // -----------------------------------------------------------------------
    // COMPLETION — D-12 celebratory ascending Do-Mi-Sol (C5-E5-G5), ~1200ms.
    //   Three notes overlapping with shared reverb so the tail of each note
    //   bleeds into the onset of the next (no hard cut between notes).
    //   Each note: Risset bell + woody FM + metallic FM at increasing brightness.
    //   Final reverb: 200ms tail, 25% wet, fb=0.84.
    // -----------------------------------------------------------------------
    {
        let note_ms = 380;
        let n_per_note = note_ms as usize * SR as usize / 1000;
        let stride_ms = 320; // notes start every 320ms (overlapping by 60ms)
        let stride_n = stride_ms as usize * SR as usize / 1000;
        let total_n = stride_n * 2 + n_per_note;
        let mut comp = vec![0.0_f32; total_n];

        // Note 1: C5 (523.25 Hz), starts at t=0
        let n1 = build_tier_voice(523.25, note_ms, 1.0, 0.55, 0.20, SR);
        for (i, &s) in n1.iter().enumerate() {
            comp[i] += s;
        }
        // Note 2: E5 (659.25 Hz), starts at t=320ms
        let n2 = build_tier_voice(659.25, note_ms, 1.0, 0.55, 0.25, SR);
        for (i, &s) in n2.iter().enumerate() {
            comp[stride_n + i] += s;
        }
        // Note 3: G5 (784.00 Hz), starts at t=640ms — extra shimmer (4·f0 partial via metallic FM)
        let n3 = build_tier_voice(784.0, note_ms, 1.0, 0.60, 0.40, SR);
        for (i, &s) in n3.iter().enumerate() {
            comp[stride_n * 2 + i] += s;
        }

        soft_clip(&mut comp, 1.8);
        let mut wet = apply_reverb(&comp, 0.25, 0.84, SR, 200);
        dc_block(&mut wet);
        cosine_tail_fade(&mut wet, 12.0, SR);
        normalize_to_dbfs(&mut wet, -2.0);
        write_wav("assets/sfx/popup-100pct.wav", &to_i16(&wet), SR);
    }

    println!(
        "ok — generated assets/sfx/popup-standard.wav (~480ms, -3dBFS), \
         popup-rare.wav (~750ms, -2dBFS), popup-100pct.wav (~1200ms, -2dBFS) \
         using Risset bell + dual-FM (C:M=1.4142 + 1:3.5) + Schroeder reverb pipeline"
    );
}
