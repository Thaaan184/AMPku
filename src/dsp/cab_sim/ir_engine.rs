//! FIR Cabinet Impulse Response (IR) convolution engine.
//!
//! Provides zero-latency direct time-domain FIR convolution for stereo audio frames
//! with zero heap allocations during streaming audio processing.

use crate::core::frame::AudioFrame;
use crate::dsp::cab_sim::factory_irs::{generate_factory_ir, IR_LENGTH};
use crate::dsp::traits::AudioEffect;

const BUFFER_LENGTH: usize = IR_LENGTH * 2;

/// Factory speaker cabinet impulse response profiles.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum CabinetProfile {
    /// Celestion Vintage 30 4x12: punchy upper-mid spike for rock and modern metal.
    #[default]
    CelestionV30,
    /// Fender Twin Reverb 2x12: sparkling highs, scooped mids, and vintage American clean chime.
    FenderTwin,
    /// Marshall 1960A 4x12: classic British crunch with resonant low thump and aggressive bite.
    Marshall1960,
    /// Acoustic Simulator: piezo electric body transformation with resonant cavity acoustics.
    AcousticSim,
}

impl CabinetProfile {
    /// All available factory cabinet profiles.
    pub const ALL: [CabinetProfile; 4] = [
        CabinetProfile::CelestionV30,
        CabinetProfile::FenderTwin,
        CabinetProfile::Marshall1960,
        CabinetProfile::AcousticSim,
    ];

    /// Return all factory cabinet profiles as a slice.
    pub fn all() -> &'static [CabinetProfile] {
        &Self::ALL
    }

    /// Display name of the cabinet profile.
    pub fn name(&self) -> &'static str {
        match self {
            CabinetProfile::CelestionV30 => "Celestion V30 4x12",
            CabinetProfile::FenderTwin => "Fender Twin Reverb 2x12",
            CabinetProfile::Marshall1960 => "Marshall 1960A 4x12",
            CabinetProfile::AcousticSim => "Acoustic Simulator Cab",
        }
    }

    /// Description of the tonal characteristics of this cabinet profile.
    pub fn description(&self) -> &'static str {
        match self {
            CabinetProfile::CelestionV30 => {
                "Punchy upper-mid spike and tight low-end for rock and modern metal"
            }
            CabinetProfile::FenderTwin => {
                "Vintage sparkling American clean with scooped mids and airy treble"
            }
            CabinetProfile::Marshall1960 => {
                "Aggressive British crunch with low-end cabinet thump and bite"
            }
            CabinetProfile::AcousticSim => {
                "Acoustic guitar body simulation with resonant soundboard and piezo de-quack"
            }
        }
    }
}

impl std::fmt::Display for CabinetProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Helper struct for serialization and deserialization of `CabinetSim` presets.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct CabinetSimConfig {
    profile: CabinetProfile,
    #[serde(default = "default_mix")]
    mix: f32,
    #[serde(default)]
    output_gain_db: f32,
    #[serde(default = "default_sample_rate")]
    sample_rate: f32,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_mix() -> f32 {
    1.0
}
fn default_sample_rate() -> f32 {
    48000.0
}
fn default_enabled() -> bool {
    true
}

/// Zero-latency FIR cabinet impulse response convolution engine.
///
/// Employs pre-allocated double-sized circular history buffers for continuous
/// cache-friendly SIMD dot-product direct time-domain FIR convolution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(from = "CabinetSimConfig", into = "CabinetSimConfig")]
pub struct CabinetSim {
    profile: CabinetProfile,
    mix: f32,
    output_gain_db: f32,
    sample_rate: f32,
    enabled: bool,
    gain_linear: f32,
    ir: [f32; IR_LENGTH],
    ir_reversed: [f32; IR_LENGTH],
    history_l: [f32; BUFFER_LENGTH],
    history_r: [f32; BUFFER_LENGTH],
    write_pos: usize,
}

impl CabinetSim {
    /// Create a new `CabinetSim` with default 48 kHz sample rate.
    pub fn new(profile: CabinetProfile) -> Self {
        Self::with_sample_rate(profile, 48000.0)
    }

    /// Create a new `CabinetSim` with an explicit sample rate.
    pub fn with_sample_rate(profile: CabinetProfile, sample_rate: f32) -> Self {
        let sample_rate = if sample_rate.is_finite() && sample_rate > 1000.0 {
            sample_rate
        } else {
            48000.0
        };

        let ir = generate_factory_ir(profile, sample_rate);
        let mut ir_reversed = [0.0f32; IR_LENGTH];
        for i in 0..IR_LENGTH {
            ir_reversed[i] = ir[IR_LENGTH - 1 - i];
        }

        Self {
            profile,
            mix: 1.0,
            output_gain_db: 0.0,
            sample_rate,
            enabled: true,
            gain_linear: 1.0,
            ir,
            ir_reversed,
            history_l: [0.0f32; BUFFER_LENGTH],
            history_r: [0.0f32; BUFFER_LENGTH],
            write_pos: 0,
        }
    }

    /// Return the active cabinet profile.
    pub fn profile(&self) -> CabinetProfile {
        self.profile
    }

    /// Switch to a new factory cabinet profile and update the active IR.
    pub fn set_profile(&mut self, profile: CabinetProfile) {
        self.profile = profile;
        self.ir = generate_factory_ir(profile, self.sample_rate);
        for i in 0..IR_LENGTH {
            self.ir_reversed[i] = self.ir[IR_LENGTH - 1 - i];
        }
        self.reset();
    }

    /// Return the dry/wet mix ratio (0.0 = 100% dry, 1.0 = 100% wet).
    pub fn mix(&self) -> f32 {
        self.mix
    }

    /// Set the dry/wet mix ratio (clamped to `[0.0, 1.0]`).
    pub fn set_mix(&mut self, mix: f32) {
        if mix.is_finite() {
            self.mix = mix.clamp(0.0, 1.0);
        }
    }

    /// Return the output gain in decibels.
    pub fn output_gain_db(&self) -> f32 {
        self.output_gain_db
    }

    /// Set the output gain in decibels (clamped to `[-36.0, 36.0]`).
    pub fn set_output_gain_db(&mut self, gain_db: f32) {
        if gain_db.is_finite() {
            let clamped = gain_db.clamp(-36.0, 36.0);
            self.output_gain_db = clamped;
            self.gain_linear = 10.0f32.powf(clamped / 20.0);
        }
    }

    /// Return the configured sample rate in Hz.
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Update the sample rate and regenerate the active profile impulse response.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if sample_rate.is_finite() && sample_rate > 1000.0 {
            self.sample_rate = sample_rate;
            self.ir = generate_factory_ir(self.profile, sample_rate);
            for i in 0..IR_LENGTH {
                self.ir_reversed[i] = self.ir[IR_LENGTH - 1 - i];
            }
            self.reset();
        }
    }

    /// Load custom impulse response samples into the pre-allocated convolution engine.
    ///
    /// The IR will be normalized and truncated or zero-padded to `IR_LENGTH`.
    /// Zero heap allocations occur during this operation or subsequent processing.
    pub fn load_custom_ir(&mut self, samples: &[f32]) {
        let mut new_ir = [0.0f32; IR_LENGTH];
        let copy_len = samples.len().min(IR_LENGTH);
        new_ir[..copy_len].copy_from_slice(&samples[..copy_len]);

        let mut peak = 0.0f32;
        for &s in new_ir.iter() {
            if s.abs() > peak {
                peak = s.abs();
            }
        }
        if peak > 1e-6 {
            let scale = 0.5 / peak;
            for s in new_ir.iter_mut() {
                *s *= scale;
            }
        }

        self.ir = new_ir;
        for i in 0..IR_LENGTH {
            self.ir_reversed[i] = self.ir[IR_LENGTH - 1 - i];
        }
        self.reset();
    }

    /// Return a reference to the active 512-sample impulse response array.
    pub fn impulse_response(&self) -> &[f32; IR_LENGTH] {
        &self.ir
    }

    /// Return display name of the active cabinet profile.
    pub fn active_profile_name(&self) -> &'static str {
        self.profile.name()
    }
}

impl Default for CabinetSim {
    fn default() -> Self {
        Self::new(CabinetProfile::default())
    }
}

impl PartialEq for CabinetSim {
    fn eq(&self, other: &Self) -> bool {
        self.profile == other.profile
            && (self.mix - other.mix).abs() < 1e-6
            && (self.output_gain_db - other.output_gain_db).abs() < 1e-6
            && (self.sample_rate - other.sample_rate).abs() < 1e-6
            && self.enabled == other.enabled
    }
}

impl From<CabinetSimConfig> for CabinetSim {
    fn from(cfg: CabinetSimConfig) -> Self {
        let mut sim = CabinetSim::with_sample_rate(cfg.profile, cfg.sample_rate);
        sim.set_mix(cfg.mix);
        sim.set_output_gain_db(cfg.output_gain_db);
        sim.set_enabled(cfg.enabled);
        sim
    }
}

impl From<CabinetSim> for CabinetSimConfig {
    fn from(sim: CabinetSim) -> Self {
        CabinetSimConfig {
            profile: sim.profile,
            mix: sim.mix,
            output_gain_db: sim.output_gain_db,
            sample_rate: sim.sample_rate,
            enabled: sim.enabled,
        }
    }
}

impl AudioEffect for CabinetSim {
    #[inline]
    fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame {
        if !self.enabled {
            return frame;
        }

        let left = if frame.left.is_finite() {
            frame.left
        } else {
            0.0
        };
        let right = if frame.right.is_finite() {
            frame.right
        } else {
            0.0
        };

        let pos = self.write_pos;
        self.history_l[pos] = left;
        self.history_l[pos + IR_LENGTH] = left;
        self.history_r[pos] = right;
        self.history_r[pos + IR_LENGTH] = right;

        let start = pos + 1;
        let slice_l = &self.history_l[start..start + IR_LENGTH];
        let slice_r = &self.history_r[start..start + IR_LENGTH];

        let mut sum_l = 0.0f32;
        let mut sum_r = 0.0f32;
        for i in 0..IR_LENGTH {
            sum_l += slice_l[i] * self.ir_reversed[i];
            sum_r += slice_r[i] * self.ir_reversed[i];
        }

        self.write_pos = if pos + 1 >= IR_LENGTH { 0 } else { pos + 1 };

        let wet_l = sum_l * self.gain_linear;
        let wet_r = sum_r * self.gain_linear;

        let mut out_l = left * (1.0 - self.mix) + wet_l * self.mix;
        let mut out_r = right * (1.0 - self.mix) + wet_r * self.mix;

        // Flush subnormal values to prevent denormal arithmetic stalls
        if out_l.abs() < 1e-15 {
            out_l = 0.0;
        }
        if out_r.abs() < 1e-15 {
            out_r = 0.0;
        }

        AudioFrame::new(out_l, out_r)
    }

    fn reset(&mut self) {
        self.history_l.fill(0.0);
        self.history_r.fill(0.0);
        self.write_pos = 0;
    }

    fn name(&self) -> &'static str {
        "Cabinet IR Simulator"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}
