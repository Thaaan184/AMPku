//! Algorithmic stereo Schroeder / Freeverb reverberator featuring 8 parallel
//! comb filters and 4 series allpass filters per channel with stereo decorrelation.

use crate::core::frame::AudioFrame;
use crate::dsp::traits::AudioEffect;

const NUM_COMBS: usize = 8;
const NUM_ALLPASSES: usize = 4;

// Classic Freeverb comb filter base lengths in samples at 44.1 kHz
const COMB_TUNING_L: [usize; NUM_COMBS] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
// Classic Freeverb allpass filter base lengths in samples at 44.1 kHz
const ALLPASS_TUNING_L: [usize; NUM_ALLPASSES] = [556, 441, 341, 225];

// Stereo spread offset in samples to decorrelate left and right channels
const STEREO_SPREAD: usize = 23;

// Fixed gain scaling to prevent comb summing overflow
const FIXED_GAIN: f32 = 0.015;
const ALLPASS_FEEDBACK: f32 = 0.5;

/// Single low-pass feedback comb filter (LBFC).
#[derive(Debug, Clone)]
struct CombFilter {
    buffer: Vec<f32>,
    buf_size: usize,
    buf_idx: usize,
    filter_store: f32,
    damp: f32,
    feedback: f32,
}

impl CombFilter {
    fn new(size: usize, feedback: f32, damp: f32) -> Self {
        let size = size.max(1);
        Self {
            buffer: vec![0.0; size],
            buf_size: size,
            buf_idx: 0,
            filter_store: 0.0,
            damp,
            feedback,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.buf_idx];

        // 1-pole low-pass filter on feedback loop
        self.filter_store = output * (1.0 - self.damp) + self.filter_store * self.damp;

        // Subnormal / denormal flush
        if self.filter_store.abs() < 1e-15 {
            self.filter_store = 0.0;
        }

        self.buffer[self.buf_idx] = input + self.filter_store * self.feedback;
        self.buf_idx += 1;
        if self.buf_idx >= self.buf_size {
            self.buf_idx = 0;
        }

        output
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.buf_idx = 0;
        self.filter_store = 0.0;
    }

    fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback;
    }

    fn set_damp(&mut self, damp: f32) {
        self.damp = damp;
    }
}

/// Single Schroeder allpass filter with feedback coefficient.
#[derive(Debug, Clone)]
struct AllpassFilter {
    buffer: Vec<f32>,
    buf_size: usize,
    buf_idx: usize,
    feedback: f32,
}

impl AllpassFilter {
    fn new(size: usize, feedback: f32) -> Self {
        let size = size.max(1);
        Self {
            buffer: vec![0.0; size],
            buf_size: size,
            buf_idx: 0,
            feedback,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let buf_out = self.buffer[self.buf_idx];
        let output = -input + buf_out;
        self.buffer[self.buf_idx] = input + buf_out * self.feedback;

        self.buf_idx += 1;
        if self.buf_idx >= self.buf_size {
            self.buf_idx = 0;
        }

        output
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.buf_idx = 0;
    }
}

/// Serializable configuration state for `StereoReverb`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StereoReverbConfig {
    pub sample_rate: f32,
    pub room_size: f32,
    pub damping: f32,
    pub width: f32,
    pub mix: f32,
    pub enabled: bool,
}

/// Algorithmic Schroeder/Freeverb stereo reverberator featuring 8 parallel
/// comb filters and 4 series allpass filters with stereo decorrelation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(from = "StereoReverbConfig", into = "StereoReverbConfig")]
pub struct StereoReverb {
    sample_rate: f32,
    room_size: f32,
    damping: f32,
    width: f32,
    mix: f32,
    enabled: bool,

    #[serde(skip)]
    combs_l: Vec<CombFilter>,
    #[serde(skip)]
    combs_r: Vec<CombFilter>,
    #[serde(skip)]
    allpasses_l: Vec<AllpassFilter>,
    #[serde(skip)]
    allpasses_r: Vec<AllpassFilter>,
}

impl PartialEq for StereoReverb {
    fn eq(&self, other: &Self) -> bool {
        self.sample_rate == other.sample_rate
            && self.room_size == other.room_size
            && self.damping == other.damping
            && self.width == other.width
            && self.mix == other.mix
            && self.enabled == other.enabled
    }
}

impl Default for StereoReverb {
    fn default() -> Self {
        Self::new(48000.0, 0.5, 0.5, 1.0, 0.3)
    }
}

impl From<StereoReverbConfig> for StereoReverb {
    fn from(config: StereoReverbConfig) -> Self {
        let mut reverb = Self::new(
            config.sample_rate,
            config.room_size,
            config.damping,
            config.width,
            config.mix,
        );
        reverb.set_enabled(config.enabled);
        reverb
    }
}

impl From<StereoReverb> for StereoReverbConfig {
    fn from(reverb: StereoReverb) -> Self {
        StereoReverbConfig {
            sample_rate: reverb.sample_rate,
            room_size: reverb.room_size,
            damping: reverb.damping,
            width: reverb.width,
            mix: reverb.mix,
            enabled: reverb.enabled,
        }
    }
}

impl StereoReverb {
    /// Create a new StereoReverb node with room size, damping, stereo width, and mix parameters.
    pub fn new(sample_rate: f32, room_size: f32, damping: f32, width: f32, mix: f32) -> Self {
        let sr = if sample_rate.is_finite() && sample_rate > 1000.0 {
            sample_rate
        } else {
            48000.0
        };

        let room_size = if room_size.is_finite() {
            room_size.clamp(0.0, 1.0)
        } else {
            0.5
        };
        let damping = if damping.is_finite() {
            damping.clamp(0.0, 1.0)
        } else {
            0.5
        };
        let width = if width.is_finite() {
            width.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let mix = if mix.is_finite() {
            mix.clamp(0.0, 1.0)
        } else {
            0.3
        };

        let (combs_l, combs_r, allpasses_l, allpasses_r) =
            Self::build_filters(sr, room_size, damping);

        Self {
            sample_rate: sr,
            room_size,
            damping,
            width,
            mix,
            enabled: true,
            combs_l,
            combs_r,
            allpasses_l,
            allpasses_r,
        }
    }

    fn build_filters(
        sample_rate: f32,
        room_size: f32,
        damping: f32,
    ) -> (
        Vec<CombFilter>,
        Vec<CombFilter>,
        Vec<AllpassFilter>,
        Vec<AllpassFilter>,
    ) {
        let scale = sample_rate / 44100.0;
        let feedback = room_size * 0.28 + 0.70;
        let damp = damping * 0.40;

        let mut combs_l = Vec::with_capacity(NUM_COMBS);
        let mut combs_r = Vec::with_capacity(NUM_COMBS);
        for &tuning in &COMB_TUNING_L {
            let size_l = ((tuning as f32 * scale).round() as usize).max(1);
            let size_r = (((tuning + STEREO_SPREAD) as f32 * scale).round() as usize).max(1);
            combs_l.push(CombFilter::new(size_l, feedback, damp));
            combs_r.push(CombFilter::new(size_r, feedback, damp));
        }

        let mut allpasses_l = Vec::with_capacity(NUM_ALLPASSES);
        let mut allpasses_r = Vec::with_capacity(NUM_ALLPASSES);
        for &tuning in &ALLPASS_TUNING_L {
            let size_l = ((tuning as f32 * scale).round() as usize).max(1);
            let size_r = (((tuning + STEREO_SPREAD) as f32 * scale).round() as usize).max(1);
            allpasses_l.push(AllpassFilter::new(size_l, ALLPASS_FEEDBACK));
            allpasses_r.push(AllpassFilter::new(size_r, ALLPASS_FEEDBACK));
        }

        (combs_l, combs_r, allpasses_l, allpasses_r)
    }

    /// Set runtime parameters.
    pub fn set_params(&mut self, room_size: f32, damping: f32, width: f32, mix: f32) {
        self.set_room_size(room_size);
        self.set_damping(damping);
        self.set_width(width);
        self.set_mix(mix);
    }

    /// Room size in range `[0.0, 1.0]`.
    #[inline]
    pub fn room_size(&self) -> f32 {
        self.room_size
    }

    /// Set room size (feedback decay time).
    pub fn set_room_size(&mut self, room_size: f32) {
        if room_size.is_finite() {
            self.room_size = room_size.clamp(0.0, 1.0);
            let feedback = self.room_size * 0.28 + 0.70;
            for comb in self.combs_l.iter_mut() {
                comb.set_feedback(feedback);
            }
            for comb in self.combs_r.iter_mut() {
                comb.set_feedback(feedback);
            }
        }
    }

    /// High frequency damping in range `[0.0, 1.0]`.
    #[inline]
    pub fn damping(&self) -> f32 {
        self.damping
    }

    /// Set high frequency damping in feedback loops.
    pub fn set_damping(&mut self, damping: f32) {
        if damping.is_finite() {
            self.damping = damping.clamp(0.0, 1.0);
            let damp = self.damping * 0.40;
            for comb in self.combs_l.iter_mut() {
                comb.set_damp(damp);
            }
            for comb in self.combs_r.iter_mut() {
                comb.set_damp(damp);
            }
        }
    }

    /// Stereo width / spread in range `[0.0, 1.0]`.
    #[inline]
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Set stereo width (0.0 = mono reverb, 1.0 = fully decorrelated stereo).
    pub fn set_width(&mut self, width: f32) {
        if width.is_finite() {
            self.width = width.clamp(0.0, 1.0);
        }
    }

    /// Dry/wet mix in range `[0.0, 1.0]`.
    #[inline]
    pub fn mix(&self) -> f32 {
        self.mix
    }

    /// Set dry/wet mix.
    pub fn set_mix(&mut self, mix: f32) {
        if mix.is_finite() {
            self.mix = mix.clamp(0.0, 1.0);
        }
    }

    /// Current sample rate in Hz.
    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Update sample rate, rebuilding comb and allpass delay buffers.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if sample_rate.is_finite()
            && sample_rate > 1000.0
            && (sample_rate - self.sample_rate).abs() > 1.0
        {
            self.sample_rate = sample_rate;
            let (combs_l, combs_r, allpasses_l, allpasses_r) =
                Self::build_filters(sample_rate, self.room_size, self.damping);
            self.combs_l = combs_l;
            self.combs_r = combs_r;
            self.allpasses_l = allpasses_l;
            self.allpasses_r = allpasses_r;
        }
    }
}

impl AudioEffect for StereoReverb {
    #[inline]
    fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        let clean_frame = frame.sanitize();
        if !self.enabled {
            return clean_frame;
        }

        let input = clean_frame.to_mono() * FIXED_GAIN;

        // Process 8 parallel comb filters for left channel
        let mut out_l = 0.0;
        for comb in self.combs_l.iter_mut() {
            out_l += comb.process(input);
        }

        // Process 4 series allpass filters for left channel
        for allpass in self.allpasses_l.iter_mut() {
            out_l = allpass.process(out_l);
        }

        // Process 8 parallel comb filters for right channel (with stereo decorrelation)
        let mut out_r = 0.0;
        for comb in self.combs_r.iter_mut() {
            out_r += comb.process(input);
        }

        // Process 4 series allpass filters for right channel (with stereo decorrelation)
        for allpass in self.allpasses_r.iter_mut() {
            out_r = allpass.process(out_r);
        }

        // Stereo width mixing
        let wet1 = self.mix * (1.0 + self.width) * 0.5;
        let wet2 = self.mix * (1.0 - self.width) * 0.5;
        let dry = 1.0 - self.mix;

        let left = clean_frame.left * dry + out_l * wet1 + out_r * wet2;
        let right = clean_frame.right * dry + out_r * wet1 + out_l * wet2;

        AudioFrame::new(left, right)
    }

    fn reset(&mut self) {
        for comb in self.combs_l.iter_mut() {
            comb.reset();
        }
        for comb in self.combs_r.iter_mut() {
            comb.reset();
        }
        for allpass in self.allpasses_l.iter_mut() {
            allpass.reset();
        }
        for allpass in self.allpasses_r.iter_mut() {
            allpass.reset();
        }
    }

    fn name(&self) -> &'static str {
        "Stereo Reverb"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}
