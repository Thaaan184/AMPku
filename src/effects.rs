use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

// Utility for atomic f32 storage
#[inline]
pub fn store_f32(atomic: &AtomicU32, val: f32) {
    atomic.store(val.to_bits(), Ordering::Relaxed);
}

#[inline]
pub fn load_f32(atomic: &AtomicU32) -> f32 {
    f32::from_bits(atomic.load(Ordering::Relaxed))
}

#[inline]
pub fn flush_denormal(val: f32) -> f32 {
    if val.abs() < 1e-15 {
        0.0
    } else {
        val
    }
}

// ----------------------------------------------------
// 1. Wah Effect (Dynamic Auto-Wah via Envelope SVF)
// ----------------------------------------------------

pub struct WahDsp {
    envelope: f32,
    low: [f32; 2],
    band: [f32; 2],
    sample_rate: f32,
}

impl WahDsp {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            envelope: 0.0,
            low: [0.0; 2],
            band: [0.0; 2],
            sample_rate,
        }
    }

    pub fn process(&mut self, left: &mut f32, right: &mut f32, depth: f32, resonance: f32) {
        let input_mono = (*left + *right) * 0.5;
        let abs_in = input_mono.abs();

        // Envelope follower: fast attack, smooth decay
        let attack = 0.04;
        let release = 0.0008;
        if abs_in > self.envelope {
            self.envelope += attack * (abs_in - self.envelope);
        } else {
            self.envelope += release * (abs_in - self.envelope);
        }
        self.envelope = flush_denormal(self.envelope);

        // Sweep center frequency between 400 Hz (heel) and 2400 Hz (toe)
        let min_freq = 400.0f32;
        let max_freq = min_freq + depth * 2000.0;
        let target_freq =
            (min_freq + self.envelope * 4.0 * (max_freq - min_freq)).clamp(min_freq, max_freq);

        // State Variable Filter (SVF) parameters
        let f = 2.0 * (PI * target_freq / self.sample_rate).sin();
        let q = 1.0 / (0.15 + resonance * 0.80);

        for (ch, sample) in [left, right].into_iter().enumerate() {
            let input = *sample;
            let high = input - self.low[ch] - q * self.band[ch];
            self.band[ch] += f * high;
            self.low[ch] += f * self.band[ch];

            self.band[ch] = flush_denormal(self.band[ch]);
            self.low[ch] = flush_denormal(self.low[ch]);

            // Wah blend: 30% direct dry + resonant bandpass wet
            *sample = input * 0.35 + self.band[ch] * 1.4;
        }
    }
}

// ----------------------------------------------------
// 2. Clean Preamp (Sparkle & Warmth Tone Shaper)
// ----------------------------------------------------

pub struct CleanDsp {
    bright_state: [f32; 2],
    warm_state: [f32; 2],
}

impl CleanDsp {
    pub fn new() -> Self {
        Self {
            bright_state: [0.0; 2],
            warm_state: [0.0; 2],
        }
    }
}

impl Default for CleanDsp {
    fn default() -> Self {
        Self::new()
    }
}

impl CleanDsp {
    pub fn process(&mut self, left: &mut f32, right: &mut f32, bright: f32, warm: f32) {
        let bright_coeff = 0.22;
        let warm_coeff = 0.08;

        for (ch, sample) in [left, right].into_iter().enumerate() {
            let s = *sample;
            // High-pass sparkle
            self.bright_state[ch] += bright_coeff * (s - self.bright_state[ch]);
            let sparkle = s - self.bright_state[ch];

            // Low-pass body warmth
            self.warm_state[ch] += warm_coeff * (s - self.warm_state[ch]);
            let warmth = self.warm_state[ch];

            self.bright_state[ch] = flush_denormal(self.bright_state[ch]);
            self.warm_state[ch] = flush_denormal(self.warm_state[ch]);

            *sample = (s + sparkle * (bright * 1.2) + warmth * (warm * 0.8)) * 1.1;
        }
    }
}

// ----------------------------------------------------
// 3. Crunch Amp (Edge-of-Breakup Tube Saturation)
// ----------------------------------------------------

pub struct CrunchDsp {
    tone_state: [f32; 2],
    cab_state: [f32; 2],
}

impl CrunchDsp {
    pub fn new() -> Self {
        Self {
            tone_state: [0.0; 2],
            cab_state: [0.0; 2],
        }
    }
}

impl Default for CrunchDsp {
    fn default() -> Self {
        Self::new()
    }
}

impl CrunchDsp {
    pub fn process(&mut self, left: &mut f32, right: &mut f32, drive: f32, tone: f32) {
        let gain = 1.4 + drive * 8.5;
        let tone_alpha = 0.15 + tone * 0.65;
        let cab_alpha = 0.28; // ~4.5 kHz cabinet damping

        for (ch, sample) in [left, right].into_iter().enumerate() {
            let x = (*sample * gain).clamp(-3.0, 3.0);
            // Tube polynomial saturation with mild 2nd harmonic warmth
            let biased = x + 0.08 * x * x;
            let saturated = if biased.abs() < 1.0 {
                biased - (biased * biased * biased) / 3.0
            } else {
                biased.signum() * (2.0 / 3.0)
            };

            // Mid-range tone filter
            self.tone_state[ch] += tone_alpha * (saturated - self.tone_state[ch]);
            self.tone_state[ch] = flush_denormal(self.tone_state[ch]);

            // Speaker cab damping
            self.cab_state[ch] += cab_alpha * (self.tone_state[ch] - self.cab_state[ch]);
            self.cab_state[ch] = flush_denormal(self.cab_state[ch]);

            *sample = self.cab_state[ch] * 1.15;
        }
    }
}

// ----------------------------------------------------
// 4. Overdrive (Smooth Tanh Saturation)
// ----------------------------------------------------

pub struct OverdriveDsp {
    pub lp_state: [f32; 2],
}

impl OverdriveDsp {
    pub fn new() -> Self {
        Self { lp_state: [0.0; 2] }
    }
}

impl Default for OverdriveDsp {
    fn default() -> Self {
        Self::new()
    }
}

impl OverdriveDsp {
    pub fn process(&mut self, left: &mut f32, right: &mut f32, drive: f32, tone: f32) {
        let gain = 1.0 + drive * 12.0;
        let alpha = 0.1 + tone * 0.7;

        for (ch, sample) in [left, right].into_iter().enumerate() {
            let shaped = (*sample * gain).tanh();
            self.lp_state[ch] += alpha * (shaped - self.lp_state[ch]);
            self.lp_state[ch] = flush_denormal(self.lp_state[ch]);
            *sample = self.lp_state[ch];
        }
    }
}

// ----------------------------------------------------
// 5. Distortion (Asymmetric 1N4148 Diode Model)
// ----------------------------------------------------

pub struct DistortionDsp {
    pub pre_state: [f32; 2],
    pub post_state: [f32; 2],
}

impl DistortionDsp {
    pub fn new() -> Self {
        Self {
            pre_state: [0.0; 2],
            post_state: [0.0; 2],
        }
    }
}

impl Default for DistortionDsp {
    fn default() -> Self {
        Self::new()
    }
}

impl DistortionDsp {
    pub fn process(&mut self, left: &mut f32, right: &mut f32, drive: f32, tone: f32) {
        let gain = 2.0 + drive * 25.0;
        let tone_alpha = 0.15 + tone * 0.7;

        for (ch, sample) in [left, right].into_iter().enumerate() {
            let hp = *sample - self.pre_state[ch];
            self.pre_state[ch] += 0.08 * hp;
            self.pre_state[ch] = flush_denormal(self.pre_state[ch]);

            let driven = hp * gain;
            let clipped = if driven > 0.0 {
                1.0 - (-driven * 2.5).exp()
            } else {
                -1.0 + (driven * 1.5).exp()
            };

            self.post_state[ch] += tone_alpha * (clipped - self.post_state[ch]);
            self.post_state[ch] = flush_denormal(self.post_state[ch]);
            *sample = self.post_state[ch] * 0.8;
        }
    }
}

// ----------------------------------------------------
// 6. Delay (Stereo Ping-Pong Delay Line)
// ----------------------------------------------------

pub struct DelayDsp {
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    write_idx: usize,
    sample_rate: f32,
}

impl DelayDsp {
    pub fn new(max_seconds: f32, sample_rate: f32) -> Self {
        let cap = (max_seconds * sample_rate) as usize + 1024;
        Self {
            buffer_l: vec![0.0; cap],
            buffer_r: vec![0.0; cap],
            write_idx: 0,
            sample_rate,
        }
    }

    pub fn process(
        &mut self,
        left: &mut f32,
        right: &mut f32,
        time_ms: f32,
        feedback: f32,
        mix: f32,
    ) {
        let delay_frames_l = ((time_ms / 1000.0) * self.sample_rate) as usize;
        let delay_frames_r = (((time_ms * 1.15) / 1000.0) * self.sample_rate) as usize;

        let cap = self.buffer_l.len();
        let read_idx_l = (self.write_idx + cap - (delay_frames_l % cap)) % cap;
        let read_idx_r = (self.write_idx + cap - (delay_frames_r % cap)) % cap;

        let delayed_l = self.buffer_l[read_idx_l];
        let delayed_r = self.buffer_r[read_idx_r];

        self.buffer_l[self.write_idx] = flush_denormal(*left + delayed_l * feedback);
        self.buffer_r[self.write_idx] = flush_denormal(*right + delayed_r * feedback);

        self.write_idx = (self.write_idx + 1) % cap;

        *left = *left * (1.0 - mix) + delayed_l * mix;
        *right = *right * (1.0 - mix) + delayed_r * mix;
    }
}

// ----------------------------------------------------
// Effect Parameters (Thread-safe shared state)
// ----------------------------------------------------

pub struct EffectParams {
    pub gain: AtomicU32,

    pub clean_enabled: AtomicBool,
    pub clean_bright: AtomicU32,
    pub clean_warmth: AtomicU32,

    pub crunch_enabled: AtomicBool,
    pub crunch_drive: AtomicU32,
    pub crunch_tone: AtomicU32,

    pub overdrive_enabled: AtomicBool,
    pub overdrive_drive: AtomicU32,
    pub overdrive_tone: AtomicU32,

    pub distortion_enabled: AtomicBool,
    pub distortion_drive: AtomicU32,
    pub distortion_tone: AtomicU32,

    pub wah_enabled: AtomicBool,
    pub wah_depth: AtomicU32,
    pub wah_resonance: AtomicU32,

    pub delay_enabled: AtomicBool,
    pub delay_time_ms: AtomicU32,
    pub delay_feedback: AtomicU32,
    pub delay_mix: AtomicU32,

    pub global_bypass: AtomicBool,
    pub mute: AtomicBool,

    pub input_peak: AtomicU32,
    pub output_peak: AtomicU32,
}

impl Default for EffectParams {
    fn default() -> Self {
        Self {
            gain: AtomicU32::new(1.0f32.to_bits()),

            clean_enabled: AtomicBool::new(false),
            clean_bright: AtomicU32::new(0.50f32.to_bits()),
            clean_warmth: AtomicU32::new(0.40f32.to_bits()),

            crunch_enabled: AtomicBool::new(false),
            crunch_drive: AtomicU32::new(0.45f32.to_bits()),
            crunch_tone: AtomicU32::new(0.55f32.to_bits()),

            overdrive_enabled: AtomicBool::new(true),
            overdrive_drive: AtomicU32::new(0.40f32.to_bits()),
            overdrive_tone: AtomicU32::new(0.60f32.to_bits()),

            distortion_enabled: AtomicBool::new(false),
            distortion_drive: AtomicU32::new(0.50f32.to_bits()),
            distortion_tone: AtomicU32::new(0.50f32.to_bits()),

            wah_enabled: AtomicBool::new(false),
            wah_depth: AtomicU32::new(0.70f32.to_bits()),
            wah_resonance: AtomicU32::new(0.65f32.to_bits()),

            delay_enabled: AtomicBool::new(true),
            delay_time_ms: AtomicU32::new(320.0f32.to_bits()),
            delay_feedback: AtomicU32::new(0.35f32.to_bits()),
            delay_mix: AtomicU32::new(0.25f32.to_bits()),

            global_bypass: AtomicBool::new(false),
            mute: AtomicBool::new(false),

            input_peak: AtomicU32::new(0.0f32.to_bits()),
            output_peak: AtomicU32::new(0.0f32.to_bits()),
        }
    }
}

// ----------------------------------------------------
// Effect Engine (Carried inside audio playback thread)
// ----------------------------------------------------

pub struct EffectEngine {
    pub params: Arc<EffectParams>,
    wah: WahDsp,
    clean: CleanDsp,
    crunch: CrunchDsp,
    overdrive: OverdriveDsp,
    distortion: DistortionDsp,
    delay: DelayDsp,
}

impl EffectEngine {
    pub fn new(params: Arc<EffectParams>, sample_rate: f32) -> Self {
        Self {
            params,
            wah: WahDsp::new(sample_rate),
            clean: CleanDsp::new(),
            crunch: CrunchDsp::new(),
            overdrive: OverdriveDsp::new(),
            distortion: DistortionDsp::new(),
            delay: DelayDsp::new(2.0, sample_rate),
        }
    }

    /// Process mono guitar input into interleaved stereo output.
    pub fn process_mono_to_stereo(&mut self, in_mono: &[f32], out_stereo: &mut [f32]) {
        if self.params.mute.load(Ordering::Relaxed) {
            out_stereo.fill(0.0);
            store_f32(&self.params.input_peak, 0.0);
            store_f32(&self.params.output_peak, 0.0);
            return;
        }

        let bypass = self.params.global_bypass.load(Ordering::Relaxed);
        let gain = load_f32(&self.params.gain);

        let wah_enabled = self.params.wah_enabled.load(Ordering::Relaxed);
        let wah_depth = load_f32(&self.params.wah_depth);
        let wah_res = load_f32(&self.params.wah_resonance);

        let clean_enabled = self.params.clean_enabled.load(Ordering::Relaxed);
        let clean_bright = load_f32(&self.params.clean_bright);
        let clean_warmth = load_f32(&self.params.clean_warmth);

        let crunch_enabled = self.params.crunch_enabled.load(Ordering::Relaxed);
        let crunch_drive = load_f32(&self.params.crunch_drive);
        let crunch_tone = load_f32(&self.params.crunch_tone);

        let od_enabled = self.params.overdrive_enabled.load(Ordering::Relaxed);
        let od_drive = load_f32(&self.params.overdrive_drive);
        let od_tone = load_f32(&self.params.overdrive_tone);

        let dist_enabled = self.params.distortion_enabled.load(Ordering::Relaxed);
        let dist_drive = load_f32(&self.params.distortion_drive);
        let dist_tone = load_f32(&self.params.distortion_tone);

        let delay_enabled = self.params.delay_enabled.load(Ordering::Relaxed);
        let delay_time = load_f32(&self.params.delay_time_ms);
        let delay_fb = load_f32(&self.params.delay_feedback);
        let delay_mix = load_f32(&self.params.delay_mix);

        let mut max_in = 0.0f32;
        let mut max_out = 0.0f32;

        let frames = in_mono.len().min(out_stereo.len() / 2);

        for i in 0..frames {
            let sample = in_mono[i] * gain;
            max_in = max_in.max(sample.abs());

            let (mut left, mut right) = (sample, sample);

            if !bypass {
                // 1. Wah
                if wah_enabled {
                    self.wah.process(&mut left, &mut right, wah_depth, wah_res);
                }
                // 2. Clean Preamp
                if clean_enabled {
                    self.clean
                        .process(&mut left, &mut right, clean_bright, clean_warmth);
                }
                // 3. Crunch Amp
                if crunch_enabled {
                    self.crunch
                        .process(&mut left, &mut right, crunch_drive, crunch_tone);
                }
                // 4. Overdrive
                if od_enabled {
                    self.overdrive
                        .process(&mut left, &mut right, od_drive, od_tone);
                }
                // 5. Distortion
                if dist_enabled {
                    self.distortion
                        .process(&mut left, &mut right, dist_drive, dist_tone);
                }
                // 6. Delay
                if delay_enabled {
                    self.delay
                        .process(&mut left, &mut right, delay_time, delay_fb, delay_mix);
                }

                // Output brickwall limiter clamp
                left = left.clamp(-0.98, 0.98);
                right = right.clamp(-0.98, 0.98);
            }

            max_out = max_out.max(left.abs().max(right.abs()));

            out_stereo[i * 2] = left;
            out_stereo[i * 2 + 1] = right;
        }

        // Fill remaining output frames with silence if in_mono was shorter
        for sample in out_stereo.iter_mut().skip(frames * 2) {
            *sample = 0.0;
        }

        // Update atomic peak meters
        store_f32(&self.params.input_peak, max_in);
        store_f32(&self.params.output_peak, max_out);
    }
}
