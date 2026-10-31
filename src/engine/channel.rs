//! Channel strip engine with volume, pan matrix, mute/solo flags, mono toggle, and lock-free VU metering.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::atomic_float::AtomicF32;
use crate::core::frame::AudioFrame;
use crate::dsp::traits::AudioEffect;
use crate::engine::chain::EffectChain;

/// Convert linear amplitude ratio to decibels (dBFS).
#[inline]
pub fn linear_to_db(linear: f32) -> f32 {
    let abs_val = linear.abs();
    if abs_val <= 1e-6 {
        -120.0
    } else {
        20.0 * abs_val.log10()
    }
}

/// Convert decibels (dBFS) to linear amplitude ratio.
#[inline]
pub fn db_to_linear(db: f32) -> f32 {
    if db <= -100.0 {
        0.0
    } else {
        10.0_f32.powf(db / 20.0)
    }
}

/// Stereo panning law calculation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanLaw {
    /// Constant-power (-3 dB center) pan law: preserves perceived acoustic energy across pan positions.
    #[default]
    ConstantPower,
    /// Linear balance (0 dB center, attenuation on opposite channel).
    Linear,
}

/// Calculate stereo left and right channel gain coefficients for a given pan position (-1.0 = Left, +1.0 = Right).
#[inline]
pub fn calculate_pan_gains(pan: f32, law: PanLaw) -> (f32, f32) {
    let pan = pan.clamp(-1.0, 1.0);
    match law {
        PanLaw::ConstantPower => {
            if pan <= -1.0 {
                (1.0, 0.0)
            } else if pan >= 1.0 {
                (0.0, 1.0)
            } else {
                // angle in (0, pi/2)
                let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
                (angle.cos(), angle.sin())
            }
        }
        PanLaw::Linear => {
            let left = if pan > 0.0 { 1.0 - pan } else { 1.0 };
            let right = if pan < 0.0 { 1.0 + pan } else { 1.0 };
            (left, right)
        }
    }
}

/// Real-time lock-free stereo VU meter measuring peak and RMS levels with decay ballistics.
#[derive(Debug)]
pub struct ChannelMeter {
    peak_left: AtomicF32,
    peak_right: AtomicF32,
    rms_left: AtomicF32,
    rms_right: AtomicF32,
    max_peak_left: AtomicF32,
    max_peak_right: AtomicF32,
    clipped: AtomicBool,
}

impl ChannelMeter {
    /// Create a new channel meter with zeroed levels.
    pub fn new() -> Self {
        Self {
            peak_left: AtomicF32::new(0.0),
            peak_right: AtomicF32::new(0.0),
            rms_left: AtomicF32::new(0.0),
            rms_right: AtomicF32::new(0.0),
            max_peak_left: AtomicF32::new(0.0),
            max_peak_right: AtomicF32::new(0.0),
            clipped: AtomicBool::new(false),
        }
    }

    /// Current instantaneous decayed peak values `(left, right)`.
    #[inline]
    pub fn peak(&self) -> (f32, f32) {
        (self.peak_left.get(), self.peak_right.get())
    }

    /// Peak level of left channel.
    #[inline]
    pub fn peak_left(&self) -> f32 {
        self.peak_left.get()
    }

    /// Peak level of right channel.
    #[inline]
    pub fn peak_right(&self) -> f32 {
        self.peak_right.get()
    }

    /// Current smoothed RMS values `(left, right)`.
    #[inline]
    pub fn rms(&self) -> (f32, f32) {
        (self.rms_left.get(), self.rms_right.get())
    }

    /// RMS level of left channel.
    #[inline]
    pub fn rms_left(&self) -> f32 {
        self.rms_left.get()
    }

    /// RMS level of right channel.
    #[inline]
    pub fn rms_right(&self) -> f32 {
        self.rms_right.get()
    }

    /// Maximum historical peak level hold `(left, right)`.
    #[inline]
    pub fn max_peak(&self) -> (f32, f32) {
        (self.max_peak_left.get(), self.max_peak_right.get())
    }

    /// Current peak levels in decibels `(left_db, right_db)`.
    #[inline]
    pub fn peak_db(&self) -> (f32, f32) {
        (
            linear_to_db(self.peak_left.get()),
            linear_to_db(self.peak_right.get()),
        )
    }

    /// Current RMS levels in decibels `(left_db, right_db)`.
    #[inline]
    pub fn rms_db(&self) -> (f32, f32) {
        (
            linear_to_db(self.rms_left.get()),
            linear_to_db(self.rms_right.get()),
        )
    }

    /// Return true if any sample exceeded 0 dBFS (amplitude >= 1.0).
    #[inline]
    pub fn is_clipped(&self) -> bool {
        self.clipped.load(Ordering::Relaxed)
    }

    /// Reset digital clipping flag.
    #[inline]
    pub fn reset_clip(&self) {
        self.clipped.store(false, Ordering::Relaxed);
    }

    /// Reset historical maximum peak hold.
    #[inline]
    pub fn reset_max_peak(&self) {
        self.max_peak_left.set(0.0);
        self.max_peak_right.set(0.0);
    }

    /// Reset all meter values and clipping indicators to zero.
    pub fn reset(&self) {
        self.peak_left.set(0.0);
        self.peak_right.set(0.0);
        self.rms_left.set(0.0);
        self.rms_right.set(0.0);
        self.max_peak_left.set(0.0);
        self.max_peak_right.set(0.0);
        self.reset_clip();
    }

    /// Update peak and RMS levels with a single audio frame (real-time safe, non-blocking).
    pub fn update_frame(&self, frame: AudioFrame, sample_rate: f32) {
        let sr = if sample_rate > 0.0 {
            sample_rate
        } else {
            48000.0
        };
        // 300 ms time constant ballistics
        let decay = (-1.0 / (0.3 * sr)).exp();
        let alpha = 1.0 - decay;

        let abs_l = frame.left.abs();
        let abs_r = frame.right.abs();

        if abs_l >= 1.0 || abs_r >= 1.0 {
            self.clipped.store(true, Ordering::Relaxed);
        }
        self.max_peak_left.update_max(abs_l);
        self.max_peak_right.update_max(abs_r);

        let prev_peak_l = self.peak_left.get();
        let prev_peak_r = self.peak_right.get();
        let mut new_peak_l = abs_l.max(prev_peak_l * decay);
        let mut new_peak_r = abs_r.max(prev_peak_r * decay);
        if new_peak_l < 1e-15 {
            new_peak_l = 0.0;
        }
        if new_peak_r < 1e-15 {
            new_peak_r = 0.0;
        }
        self.peak_left.set(new_peak_l);
        self.peak_right.set(new_peak_r);

        let sq_l = abs_l * abs_l;
        let sq_r = abs_r * abs_r;
        let prev_rms_l = self.rms_left.get();
        let prev_rms_r = self.rms_right.get();
        let prev_ms_l = prev_rms_l * prev_rms_l;
        let prev_ms_r = prev_rms_r * prev_rms_r;
        let mut new_ms_l = prev_ms_l + alpha * (sq_l - prev_ms_l);
        let mut new_ms_r = prev_ms_r + alpha * (sq_r - prev_ms_r);
        if new_ms_l < 1e-15 {
            new_ms_l = 0.0;
        }
        if new_ms_r < 1e-15 {
            new_ms_r = 0.0;
        }
        self.rms_left.set(new_ms_l.sqrt());
        self.rms_right.set(new_ms_r.sqrt());
    }

    /// Update peak and RMS levels with an audio block (real-time safe, non-blocking).
    pub fn update_block(&self, frames: &[AudioFrame], sample_rate: f32) {
        if frames.is_empty() {
            return;
        }
        let sr = if sample_rate > 0.0 {
            sample_rate
        } else {
            48000.0
        };
        let n = frames.len() as f32;
        let block_decay = (-n / (0.3 * sr)).exp();
        let alpha = 1.0 - block_decay;

        let mut peak_l: f32 = 0.0;
        let mut peak_r: f32 = 0.0;
        let mut sum_sq_l: f32 = 0.0;
        let mut sum_sq_r: f32 = 0.0;

        for f in frames {
            let abs_l = f.left.abs();
            let abs_r = f.right.abs();
            if abs_l > peak_l {
                peak_l = abs_l;
            }
            if abs_r > peak_r {
                peak_r = abs_r;
            }
            sum_sq_l += abs_l * abs_l;
            sum_sq_r += abs_r * abs_r;
        }

        if peak_l >= 1.0 || peak_r >= 1.0 {
            self.clipped.store(true, Ordering::Relaxed);
        }
        self.max_peak_left.update_max(peak_l);
        self.max_peak_right.update_max(peak_r);

        let prev_peak_l = self.peak_left.get();
        let prev_peak_r = self.peak_right.get();
        let mut new_peak_l = peak_l.max(prev_peak_l * block_decay);
        let mut new_peak_r = peak_r.max(prev_peak_r * block_decay);
        if new_peak_l < 1e-15 {
            new_peak_l = 0.0;
        }
        if new_peak_r < 1e-15 {
            new_peak_r = 0.0;
        }
        self.peak_left.set(new_peak_l);
        self.peak_right.set(new_peak_r);

        let block_ms_l = sum_sq_l / n;
        let block_ms_r = sum_sq_r / n;
        let prev_rms_l = self.rms_left.get();
        let prev_rms_r = self.rms_right.get();
        let prev_ms_l = prev_rms_l * prev_rms_l;
        let prev_ms_r = prev_rms_r * prev_rms_r;
        let mut new_ms_l = prev_ms_l + alpha * (block_ms_l - prev_ms_l);
        let mut new_ms_r = prev_ms_r + alpha * (block_ms_r - prev_ms_r);
        if new_ms_l < 1e-15 {
            new_ms_l = 0.0;
        }
        if new_ms_r < 1e-15 {
            new_ms_r = 0.0;
        }
        self.rms_left.set(new_ms_l.sqrt());
        self.rms_right.set(new_ms_r.sqrt());
    }
}

impl Default for ChannelMeter {
    fn default() -> Self {
        Self::new()
    }
}

/// Channel strip audio engine component containing effect chain, gain, pan, mute, solo, and VU meter.
pub struct ChannelStrip {
    /// Human-readable channel identifier (e.g. "Guitar", "Mic").
    pub name: String,
    /// Dedicated effect rack chain for this channel strip.
    pub chain: EffectChain,
    gain: AtomicF32,
    pan: AtomicF32,
    pan_law: PanLaw,
    muted: AtomicBool,
    solo: AtomicBool,
    mono: AtomicBool,
    meter: ChannelMeter,
    sample_rate: AtomicF32,
}

impl ChannelStrip {
    /// Create a new channel strip with unity gain (0.0 dB), centered pan, and 48 kHz sample rate.
    pub fn new() -> Self {
        Self::with_name("Channel")
    }

    /// Create a new channel strip with a custom channel name.
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            chain: EffectChain::new(),
            gain: AtomicF32::new(1.0),
            pan: AtomicF32::new(0.0),
            pan_law: PanLaw::default(),
            muted: AtomicBool::new(false),
            solo: AtomicBool::new(false),
            mono: AtomicBool::new(false),
            meter: ChannelMeter::new(),
            sample_rate: AtomicF32::new(48000.0),
        }
    }

    /// Create a new channel strip with custom name and sample rate.
    pub fn with_sample_rate(name: impl Into<String>, sample_rate: f32) -> Self {
        let strip = Self::with_name(name);
        strip.sample_rate.set(sample_rate.max(1.0));
        strip
    }

    /// Get channel display name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set channel display name.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Linear channel gain factor.
    #[inline]
    pub fn gain(&self) -> f32 {
        self.gain.get()
    }

    /// Set linear channel gain factor (clamped >= 0.0).
    #[inline]
    pub fn set_gain(&self, gain: f32) {
        self.gain.set(gain.max(0.0));
    }

    /// Channel volume in decibels (dBFS).
    #[inline]
    pub fn volume_db(&self) -> f32 {
        linear_to_db(self.gain())
    }

    /// Set channel volume in decibels (dBFS).
    #[inline]
    pub fn set_volume_db(&self, db: f32) {
        self.set_gain(db_to_linear(db));
    }

    /// Current stereo pan position in `[-1.0, 1.0]`.
    #[inline]
    pub fn pan(&self) -> f32 {
        self.pan.get()
    }

    /// Set stereo pan position in `[-1.0, 1.0]`.
    #[inline]
    pub fn set_pan(&self, pan: f32) {
        self.pan.set(pan.clamp(-1.0, 1.0));
    }

    /// Current pan law strategy.
    #[inline]
    pub fn pan_law(&self) -> PanLaw {
        self.pan_law
    }

    /// Set pan law strategy.
    #[inline]
    pub fn set_pan_law(&mut self, law: PanLaw) {
        self.pan_law = law;
    }

    /// Calculate stereo pan gain factors `(left_gain, right_gain)` according to configured pan law.
    #[inline]
    pub fn pan_gains(&self) -> (f32, f32) {
        calculate_pan_gains(self.pan(), self.pan_law)
    }

    /// Return true if channel is muted.
    #[inline]
    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }

    /// Mute or un-mute this channel.
    #[inline]
    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Relaxed);
    }

    /// Toggle channel mute state and return new state.
    pub fn toggle_mute(&self) -> bool {
        let new_state = !self.is_muted();
        self.set_muted(new_state);
        new_state
    }

    /// Return true if channel is soloed.
    #[inline]
    pub fn is_solo(&self) -> bool {
        self.solo.load(Ordering::Relaxed)
    }

    /// Set channel solo flag.
    #[inline]
    pub fn set_solo(&self, solo: bool) {
        self.solo.store(solo, Ordering::Relaxed);
    }

    /// Toggle channel solo state and return new state.
    pub fn toggle_solo(&self) -> bool {
        let new_state = !self.is_solo();
        self.set_solo(new_state);
        new_state
    }

    /// Return true if mono downmix mode is active.
    #[inline]
    pub fn is_mono(&self) -> bool {
        self.mono.load(Ordering::Relaxed)
    }

    /// Enable or disable mono downmixing mode.
    #[inline]
    pub fn set_mono(&self, mono: bool) {
        self.mono.store(mono, Ordering::Relaxed);
    }

    /// Return true if stereo mode is active (`!is_mono()`).
    #[inline]
    pub fn is_stereo(&self) -> bool {
        !self.is_mono()
    }

    /// Enable or disable stereo mode.
    #[inline]
    pub fn set_stereo(&self, stereo: bool) {
        self.set_mono(!stereo);
    }

    /// Current audio engine sample rate in Hz.
    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate.get()
    }

    /// Set audio engine sample rate in Hz.
    #[inline]
    pub fn set_sample_rate(&self, sample_rate: f32) {
        self.sample_rate.set(sample_rate.max(1.0));
    }

    /// Reference to this channel's lock-free VU meter.
    #[inline]
    pub fn meter(&self) -> &ChannelMeter {
        &self.meter
    }

    /// Get current peak levels `(left, right)`.
    #[inline]
    pub fn peak(&self) -> (f32, f32) {
        self.meter.peak()
    }

    /// Get current RMS levels `(left, right)`.
    #[inline]
    pub fn rms(&self) -> (f32, f32) {
        self.meter.rms()
    }

    /// Get current peak levels in dBFS `(left_db, right_db)`.
    #[inline]
    pub fn peak_db(&self) -> (f32, f32) {
        self.meter.peak_db()
    }

    /// Get current RMS levels in dBFS `(left_db, right_db)`.
    #[inline]
    pub fn rms_db(&self) -> (f32, f32) {
        self.meter.rms_db()
    }

    /// Return true if channel has clipped at or above 0 dBFS.
    #[inline]
    pub fn is_clipped(&self) -> bool {
        self.meter.is_clipped()
    }

    /// Reset VU meter levels and clipping indicators.
    #[inline]
    pub fn reset_meter(&self) {
        self.meter.reset();
    }

    /// Reference to the internal effect chain.
    #[inline]
    pub fn chain(&self) -> &EffectChain {
        &self.chain
    }

    /// Mutable reference to the internal effect chain.
    #[inline]
    pub fn chain_mut(&mut self) -> &mut EffectChain {
        &mut self.chain
    }

    /// Append an effect to the channel's effect chain.
    pub fn add_effect(&mut self, effect: Box<dyn AudioEffect>) -> usize {
        self.chain.add_effect(effect)
    }

    /// Remove an effect by index from the channel's effect chain.
    pub fn remove_effect(&mut self, index: usize) -> Option<Box<dyn AudioEffect>> {
        self.chain.remove_effect(index)
    }

    /// Swap two effects in the channel's effect chain.
    pub fn swap_effects(&mut self, a: usize, b: usize) -> bool {
        self.chain.swap_effects(a, b)
    }

    /// Enable or bypass an effect in the channel's effect chain.
    pub fn bypass_effect(&mut self, index: usize, bypassed: bool) -> bool {
        self.chain.bypass_effect(index, bypassed)
    }

    /// Process a single audio frame with external solo bus context.
    ///
    /// If `solo_active` is true and this channel is not soloed, or if this channel is muted,
    /// the audio output will be silence (`AudioFrame::ZERO`).
    pub fn process_frame_with_solo(
        &mut self,
        mut frame: AudioFrame,
        solo_active: bool,
    ) -> AudioFrame {
        let muted = self.is_muted();
        let solo = self.is_solo();
        let effectively_muted = muted || (solo_active && !solo);

        // Input mono downmix if enabled
        if self.is_mono() {
            frame = AudioFrame::from_mono(frame.to_mono());
        }

        // Process through modular effect chain
        frame = self.chain.process_frame(frame);

        // Ensure mono downmix persists if mono mode active
        if self.is_mono() {
            frame = AudioFrame::from_mono(frame.to_mono());
        }

        // Apply channel volume fader and stereo pan matrix
        let gain = self.gain();
        let (gain_l, gain_r) = self.pan_gains();
        frame.left *= gain * gain_l;
        frame.right *= gain * gain_r;
        frame = frame.sanitize();

        // Feed VU meter with channel post-fader level
        self.meter.update_frame(frame, self.sample_rate.get());

        // Mute silence
        if effectively_muted {
            AudioFrame::ZERO
        } else {
            frame
        }
    }

    /// Process a single audio frame through the channel strip.
    #[inline]
    pub fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        self.process_frame_with_solo(frame, false)
    }

    /// Process a block of audio frames in-place with external solo bus context.
    pub fn process_block_with_solo(&mut self, frames: &mut [AudioFrame], solo_active: bool) {
        if frames.is_empty() {
            return;
        }

        let muted = self.is_muted();
        let solo = self.is_solo();
        let effectively_muted = muted || (solo_active && !solo);
        let is_mono = self.is_mono();

        // Input mono downmix if enabled
        if is_mono {
            for f in frames.iter_mut() {
                *f = AudioFrame::from_mono(f.to_mono());
            }
        }

        // Process through modular effect chain
        self.chain.process_block(frames);

        // Ensure mono downmix persists if mono mode active
        if is_mono {
            for f in frames.iter_mut() {
                *f = AudioFrame::from_mono(f.to_mono());
            }
        }

        // Apply channel volume fader and stereo pan matrix
        let gain = self.gain();
        let (gain_l, gain_r) = self.pan_gains();
        for f in frames.iter_mut() {
            f.left *= gain * gain_l;
            f.right *= gain * gain_r;
            *f = f.sanitize();
        }

        // Feed VU meter with channel post-fader level
        self.meter.update_block(frames, self.sample_rate.get());

        // Mute silence
        if effectively_muted {
            for f in frames.iter_mut() {
                *f = AudioFrame::ZERO;
            }
        }
    }

    /// Process a block of audio frames in-place through the channel strip.
    #[inline]
    pub fn process_block(&mut self, frames: &mut [AudioFrame]) {
        self.process_block_with_solo(frames, false);
    }

    /// Reset internal effect chain state and meter registers.
    pub fn reset(&mut self) {
        self.chain.reset();
        self.meter.reset();
    }
}

impl Default for ChannelStrip {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEffect for ChannelStrip {
    #[inline]
    fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        self.process_frame(frame)
    }

    #[inline]
    fn process_block(&mut self, frames: &mut [AudioFrame]) {
        self.process_block(frames);
    }

    #[inline]
    fn reset(&mut self) {
        self.reset();
    }

    #[inline]
    fn name(&self) -> &'static str {
        "Channel Strip"
    }

    #[inline]
    fn is_enabled(&self) -> bool {
        !self.is_muted()
    }

    #[inline]
    fn set_enabled(&mut self, enabled: bool) {
        self.set_muted(!enabled);
    }
}
