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
// Effect State (DSP processing logic on audio thread)
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
        let alpha = 0.1 + tone * 0.7; // tone cutoff

        for (ch, sample) in [left, right].into_iter().enumerate() {
            let shaped = (*sample * gain).tanh();
            self.lp_state[ch] += alpha * (shaped - self.lp_state[ch]);
            self.lp_state[ch] = flush_denormal(self.lp_state[ch]);
            *sample = self.lp_state[ch];
        }
    }
}

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
            // High-pass pre-emphasis filter around 720 Hz
            let hp = *sample - self.pre_state[ch];
            self.pre_state[ch] += 0.08 * hp;
            self.pre_state[ch] = flush_denormal(self.pre_state[ch]);

            let driven = hp * gain;
            // Asymmetric diode clipping curve
            let clipped = if driven > 0.0 {
                1.0 - (-driven * 2.5).exp()
            } else {
                -1.0 + (driven * 1.5).exp()
            };

            // Post tone filter
            self.post_state[ch] += tone_alpha * (clipped - self.post_state[ch]);
            self.post_state[ch] = flush_denormal(self.post_state[ch]);
            *sample = self.post_state[ch] * 0.8;
        }
    }
}

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

    pub overdrive_enabled: AtomicBool,
    pub overdrive_drive: AtomicU32,
    pub overdrive_tone: AtomicU32,

    pub distortion_enabled: AtomicBool,
    pub distortion_drive: AtomicU32,
    pub distortion_tone: AtomicU32,

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

            overdrive_enabled: AtomicBool::new(true),
            overdrive_drive: AtomicU32::new(0.40f32.to_bits()),
            overdrive_tone: AtomicU32::new(0.60f32.to_bits()),

            distortion_enabled: AtomicBool::new(false),
            distortion_drive: AtomicU32::new(0.50f32.to_bits()),
            distortion_tone: AtomicU32::new(0.50f32.to_bits()),

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
    overdrive: OverdriveDsp,
    distortion: DistortionDsp,
    delay: DelayDsp,
}

impl EffectEngine {
    pub fn new(params: Arc<EffectParams>, sample_rate: f32) -> Self {
        Self {
            params,
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
                if od_enabled {
                    self.overdrive
                        .process(&mut left, &mut right, od_drive, od_tone);
                }
                if dist_enabled {
                    self.distortion
                        .process(&mut left, &mut right, dist_drive, dist_tone);
                }
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
