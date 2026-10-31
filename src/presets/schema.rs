//! JSON serialization schema for AMPku dual-channel studio presets.
//!
//! Presets encapsulate the complete studio configuration:
//! - Guitar Channel (Channel 1): volume, pan, mute, solo, mono, and serial effect rack.
//! - Mic Channel (Channel 2): volume, pan, mute, solo, mono, and serial effect rack.
//! - Master Bus: output volume, stereo balance, and brickwall limiter dynamics.

use serde::{Deserialize, Serialize};

use crate::dsp::cab_sim::{CabinetProfile, CabinetSim};
use crate::dsp::dynamics::{Compressor, NoiseGate};
use crate::dsp::filters::biquad::{BiquadFilter, FilterType};
use crate::dsp::saturation::{Distortion, Fuzz, Overdrive};
use crate::dsp::time::{StereoDelay, StereoReverb};
use crate::dsp::traits::AudioEffect;
use crate::dsp::vocal::{vocal_compressor, vocal_high_pass, DeEsserMode, VocalDeEsser};
use crate::engine::channel::ChannelStrip;

fn default_version() -> u32 {
    1
}

fn default_volume() -> f32 {
    1.0
}

fn default_limiter_threshold() -> f32 {
    -0.5
}

fn default_limiter_ceiling() -> f32 {
    -0.1
}

fn default_limiter_release() -> f32 {
    50.0
}

/// Full dual-channel preset data structure serializable to and from JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetData {
    /// Schema format version number (default 1).
    #[serde(default = "default_version")]
    pub version: u32,
    /// Human-readable preset name (e.g. "TexasBlues", "ModernLead").
    pub name: String,
    /// Optional descriptive notes about tone style or recommended instrument pickups.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Optional category tag (e.g. "Blues", "Rock", "Acoustic", "Vocal").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub category: String,
    /// Channel 1 (Guitar) strip configuration and effect chain.
    pub guitar_channel: ChannelPreset,
    /// Channel 2 (Microphone) strip configuration and effect chain.
    pub mic_channel: ChannelPreset,
    /// Optional master bus output level and limiter settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master: Option<MasterPreset>,
}

impl PresetData {
    /// Create a new PresetData instance with specified name and channel presets.
    pub fn new(
        name: impl Into<String>,
        guitar_channel: ChannelPreset,
        mic_channel: ChannelPreset,
    ) -> Self {
        Self {
            version: 1,
            name: name.into(),
            description: String::new(),
            category: String::new(),
            guitar_channel,
            mic_channel,
            master: None,
        }
    }

    /// Builder helper to attach a description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Builder helper to attach a category tag.
    pub fn with_category(mut self, cat: impl Into<String>) -> Self {
        self.category = cat.into();
        self
    }

    /// Builder helper to attach master bus configuration.
    pub fn with_master(mut self, master: MasterPreset) -> Self {
        self.master = Some(master);
        self
    }

    /// Serialize preset to formatted human-readable JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize preset from JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Apply this preset to Guitar and Mic channel strips at the specified sample rate.
    pub fn apply_to_channels(
        &self,
        guitar: &mut ChannelStrip,
        mic: &mut ChannelStrip,
        sample_rate: f32,
    ) -> Result<(), String> {
        self.guitar_channel.apply_to_channel(guitar, sample_rate)?;
        self.mic_channel.apply_to_channel(mic, sample_rate)?;
        Ok(())
    }
}

/// Channel strip configuration and modular effect rack preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelPreset {
    /// Linear channel gain level (1.0 = 0.0 dB unity gain).
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Stereo panning position (-1.0 = Left, 0.0 = Center, +1.0 = Right).
    #[serde(default)]
    pub pan: f32,
    /// Channel mute state.
    #[serde(default)]
    pub muted: bool,
    /// Channel solo state.
    #[serde(default)]
    pub solo: bool,
    /// Mono downmix toggle switch.
    #[serde(default)]
    pub mono: bool,
    /// Ordered list of serialized effect modules in the channel's rack.
    #[serde(default)]
    pub effects: Vec<EffectPreset>,
}

impl Default for ChannelPreset {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            mono: false,
            effects: Vec::new(),
        }
    }
}

impl ChannelPreset {
    /// Create a new channel preset with volume and pan.
    pub fn new(volume: f32, pan: f32) -> Self {
        Self {
            volume,
            pan,
            muted: false,
            solo: false,
            mono: false,
            effects: Vec::new(),
        }
    }

    /// Builder helper to set effects.
    pub fn with_effects(mut self, effects: Vec<EffectPreset>) -> Self {
        self.effects = effects;
        self
    }

    /// Append an effect configuration to the rack.
    pub fn add_effect(&mut self, effect: EffectPreset) {
        self.effects.push(effect);
    }

    /// Apply this channel preset to a target `ChannelStrip`.
    pub fn apply_to_channel(
        &self,
        channel: &mut ChannelStrip,
        sample_rate: f32,
    ) -> Result<(), String> {
        channel.set_gain(self.volume);
        channel.set_pan(self.pan);
        channel.set_muted(self.muted);
        channel.set_solo(self.solo);
        channel.set_mono(self.mono);

        channel.chain_mut().clear();
        for effect_preset in &self.effects {
            let effect_box = effect_preset.instantiate(sample_rate)?;
            channel.chain_mut().add_effect(effect_box);
        }
        Ok(())
    }
}

/// Master bus preset containing master volume and limiter configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasterPreset {
    /// Linear master output gain factor.
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Master stereo pan position.
    #[serde(default)]
    pub pan: f32,
    /// Brickwall limiter threshold in dBFS (e.g. -0.5 dB).
    #[serde(default = "default_limiter_threshold")]
    pub limiter_threshold_db: f32,
    #[serde(default = "default_limiter_ceiling")]
    /// Brickwall limiter absolute ceiling in dBFS (e.g. -0.1 dB).
    pub limiter_ceiling_db: f32,
    /// Brickwall limiter release time in milliseconds.
    #[serde(default = "default_limiter_release")]
    pub limiter_release_ms: f32,
}

impl Default for MasterPreset {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pan: 0.0,
            limiter_threshold_db: -0.5,
            limiter_ceiling_db: -0.1,
            limiter_release_ms: 50.0,
        }
    }
}

/// Serialized individual effect node in the channel rack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectPreset {
    /// Effect node type identifier (e.g. "NoiseGate", "Overdrive", "CabSimIR").
    #[serde(rename = "type")]
    pub effect_type: String,
    /// Whether this effect is currently bypassed (true = bypassed, false = active).
    #[serde(default)]
    pub bypass: bool,
    /// Arbitrary JSON key-value map representing effect slider and knob parameters.
    #[serde(default)]
    pub params: serde_json::Value,
}

impl EffectPreset {
    /// Create a generic effect preset entry.
    pub fn new(effect_type: impl Into<String>, bypass: bool, params: serde_json::Value) -> Self {
        Self {
            effect_type: effect_type.into(),
            bypass,
            params,
        }
    }

    /// Construct a NoiseGate effect preset.
    pub fn noise_gate(threshold_db: f32, attack_ms: f32, release_ms: f32, bypass: bool) -> Self {
        Self::new(
            "NoiseGate",
            bypass,
            serde_json::json!({
                "threshold_db": threshold_db,
                "attack_ms": attack_ms,
                "release_ms": release_ms,
            }),
        )
    }

    /// Construct an Overdrive effect preset.
    pub fn overdrive(drive: f32, tone: f32, level: f32, bypass: bool) -> Self {
        Self::new(
            "Overdrive",
            bypass,
            serde_json::json!({
                "drive": drive,
                "tone": tone,
                "level": level,
            }),
        )
    }

    /// Construct a Distortion effect preset.
    pub fn distortion(drive: f32, tone: f32, level: f32, bypass: bool) -> Self {
        Self::new(
            "Distortion",
            bypass,
            serde_json::json!({
                "drive": drive,
                "tone": tone,
                "level": level,
            }),
        )
    }

    /// Construct a Fuzz effect preset.
    pub fn fuzz(gain: f32, tone: f32, level: f32, bypass: bool) -> Self {
        Self::new(
            "Fuzz",
            bypass,
            serde_json::json!({
                "gain": gain,
                "tone": tone,
                "level": level,
            }),
        )
    }

    /// Construct a Compressor effect preset.
    pub fn compressor(
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        makeup_gain_db: f32,
        bypass: bool,
    ) -> Self {
        Self::new(
            "Compressor",
            bypass,
            serde_json::json!({
                "threshold_db": threshold_db,
                "ratio": ratio,
                "attack_ms": attack_ms,
                "release_ms": release_ms,
                "makeup_gain_db": makeup_gain_db,
            }),
        )
    }

    /// Construct a Cabinet Simulator IR convolution preset.
    pub fn cab_sim(
        profile: impl Into<String>,
        mix: f32,
        output_gain_db: f32,
        bypass: bool,
    ) -> Self {
        Self::new(
            "CabSimIR",
            bypass,
            serde_json::json!({
                "profile": profile.into(),
                "mix": mix,
                "output_gain_db": output_gain_db,
            }),
        )
    }

    /// Construct a Stereo Delay effect preset.
    pub fn delay(
        delay_ms: f32,
        feedback: f32,
        mix: f32,
        damping: f32,
        ping_pong: bool,
        stereo_spread: f32,
        bypass: bool,
    ) -> Self {
        Self::new(
            "Delay",
            bypass,
            serde_json::json!({
                "delay_ms": delay_ms,
                "feedback": feedback,
                "mix": mix,
                "damping": damping,
                "ping_pong": ping_pong,
                "stereo_spread": stereo_spread,
            }),
        )
    }

    /// Construct a Stereo Reverb effect preset.
    pub fn reverb(room_size: f32, damping: f32, width: f32, mix: f32, bypass: bool) -> Self {
        Self::new(
            "Reverb",
            bypass,
            serde_json::json!({
                "room_size": room_size,
                "damping": damping,
                "width": width,
                "mix": mix,
            }),
        )
    }

    /// Construct a Vocal High-Pass filter preset (80 Hz Butterworth HPF).
    pub fn vocal_high_pass(cutoff_hz: f32, bypass: bool) -> Self {
        Self::new(
            "HighPassFilter",
            bypass,
            serde_json::json!({
                "cutoff_hz": cutoff_hz,
            }),
        )
    }

    /// Construct a Vocal Compressor preset.
    pub fn vocal_compressor(
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        makeup_gain: f32,
        bypass: bool,
    ) -> Self {
        Self::new(
            "VocalCompressor",
            bypass,
            serde_json::json!({
                "threshold_db": threshold_db,
                "ratio": ratio,
                "attack_ms": attack_ms,
                "release_ms": release_ms,
                "makeup_gain": makeup_gain,
            }),
        )
    }

    /// Construct a Vocal De-Esser preset.
    pub fn vocal_de_esser(
        frequency: f32,
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        mode: impl Into<String>,
        bypass: bool,
    ) -> Self {
        Self::new(
            "VocalDeEsser",
            bypass,
            serde_json::json!({
                "frequency": frequency,
                "threshold_db": threshold_db,
                "ratio": ratio,
                "attack_ms": attack_ms,
                "release_ms": release_ms,
                "mode": mode.into(),
            }),
        )
    }

    /// Construct a parametric Biquad Filter preset.
    pub fn biquad(
        filter_type: impl Into<String>,
        freq: f32,
        q: f32,
        gain_db: f32,
        bypass: bool,
    ) -> Self {
        Self::new(
            "BiquadFilter",
            bypass,
            serde_json::json!({
                "filter_type": filter_type.into(),
                "freq": freq,
                "q": q,
                "gain_db": gain_db,
            }),
        )
    }

    /// Helper to extract an f32 parameter by key, checking aliases or falling back to default.
    fn get_f32(&self, keys: &[&str], default: f32) -> f32 {
        if let serde_json::Value::Object(map) = &self.params {
            for &key in keys {
                if let Some(val) = map.get(key) {
                    if let Some(num) = val.as_f64() {
                        return num as f32;
                    }
                }
            }
        }
        default
    }

    /// Helper to extract a bool parameter by key.
    fn get_bool(&self, keys: &[&str], default: bool) -> bool {
        if let serde_json::Value::Object(map) = &self.params {
            for &key in keys {
                if let Some(val) = map.get(key) {
                    if let Some(b) = val.as_bool() {
                        return b;
                    }
                }
            }
        }
        default
    }

    /// Helper to extract a string parameter by key.
    fn get_str<'a>(&'a self, keys: &[&str], default: &'a str) -> &'a str {
        if let serde_json::Value::Object(map) = &self.params {
            for &key in keys {
                if let Some(val) = map.get(key) {
                    if let Some(s) = val.as_str() {
                        return s;
                    }
                }
            }
        }
        default
    }

    /// Instantiate a real-time `Box<dyn AudioEffect>` DSP processor from this preset configuration.
    pub fn instantiate(&self, sample_rate: f32) -> Result<Box<dyn AudioEffect>, String> {
        let normalized_type = self.effect_type.trim().to_ascii_lowercase();

        let mut effect: Box<dyn AudioEffect> = match normalized_type.as_str() {
            "noisegate" | "noise_gate" | "noise gate" => {
                let threshold_db = self.get_f32(&["threshold_db", "threshold"], -40.0);
                let attack_ms = self.get_f32(&["attack_ms", "attack"], 5.0);
                let release_ms = self.get_f32(&["release_ms", "release"], 50.0);
                Box::new(NoiseGate::new(
                    sample_rate,
                    threshold_db,
                    attack_ms,
                    release_ms,
                ))
            }

            "overdrive" => {
                let drive = self.get_f32(&["drive"], 4.0);
                let tone = self.get_f32(&["tone"], 0.5);
                let level = self.get_f32(&["level"], 0.7);
                Box::new(Overdrive::with_sample_rate(drive, tone, level, sample_rate))
            }

            "distortion" => {
                let drive = self.get_f32(&["drive"], 8.0);
                let tone = self.get_f32(&["tone"], 0.5);
                let level = self.get_f32(&["level"], 0.7);
                Box::new(Distortion::with_sample_rate(
                    drive,
                    tone,
                    level,
                    sample_rate,
                ))
            }

            "fuzz" => {
                let gain = self.get_f32(&["gain", "drive"], 12.0);
                let tone = self.get_f32(&["tone"], 0.5);
                let level = self.get_f32(&["level"], 0.6);
                Box::new(Fuzz::with_sample_rate(gain, tone, level, sample_rate))
            }

            "compressor" => {
                let threshold_db = self.get_f32(&["threshold_db", "threshold"], -20.0);
                let ratio = self.get_f32(&["ratio"], 4.0);
                let attack_ms = self.get_f32(&["attack_ms", "attack"], 10.0);
                let release_ms = self.get_f32(&["release_ms", "release"], 100.0);
                let makeup_gain_db =
                    self.get_f32(&["makeup_gain_db", "makeup_gain", "makeup"], 0.0);
                Box::new(Compressor::new(
                    sample_rate,
                    threshold_db,
                    ratio,
                    attack_ms,
                    release_ms,
                    makeup_gain_db,
                ))
            }

            "cabsimir" | "cabsim" | "cab_sim" | "cabinetsim" | "cabinet simulator" => {
                let profile_str = self.get_str(&["profile", "cab_profile"], "CelestionV30");
                let profile = parse_cabinet_profile(profile_str);
                let mix = self.get_f32(&["mix"], 1.0);
                let output_gain_db = self.get_f32(&["output_gain_db", "gain_db"], 0.0);

                let mut cab = CabinetSim::with_sample_rate(profile, sample_rate);
                cab.set_mix(mix);
                cab.set_output_gain_db(output_gain_db);
                Box::new(cab)
            }

            "delay" | "stereodelay" | "stereo_delay" | "stereo delay" => {
                let delay_ms = self.get_f32(&["delay_ms", "time_ms", "delay"], 350.0);
                let feedback = self.get_f32(&["feedback"], 0.3);
                let mix = self.get_f32(&["mix"], 0.2);
                let damping = self.get_f32(&["damping"], 0.3);
                let ping_pong = self.get_bool(&["ping_pong", "pingpong"], false);
                let stereo_spread = self.get_f32(&["stereo_spread", "spread"], 1.0);

                Box::new(StereoDelay::with_options(
                    sample_rate,
                    delay_ms,
                    feedback,
                    mix,
                    damping,
                    ping_pong,
                    stereo_spread,
                ))
            }

            "reverb" | "stereoreverb" | "stereo_reverb" | "stereo reverb" => {
                let room_size = self.get_f32(&["room_size", "size"], 0.5);
                let damping = self.get_f32(&["damping"], 0.5);
                let width = self.get_f32(&["width"], 1.0);
                let mix = self.get_f32(&["mix"], 0.3);

                Box::new(StereoReverb::new(
                    sample_rate,
                    room_size,
                    damping,
                    width,
                    mix,
                ))
            }

            "highpassfilter" | "highpass" | "high_pass" | "hpf" => {
                let cutoff_hz = self.get_f32(&["cutoff_hz", "freq", "frequency"], 80.0);
                if (cutoff_hz - 80.0).abs() < 1e-3 {
                    Box::new(vocal_high_pass(sample_rate))
                } else {
                    Box::new(BiquadFilter::new(
                        FilterType::HighPass,
                        sample_rate,
                        cutoff_hz,
                        std::f32::consts::FRAC_1_SQRT_2,
                        0.0,
                    ))
                }
            }

            "vocalcompressor" | "vocal_compressor" => {
                let threshold_db = self.get_f32(&["threshold_db", "threshold"], -18.0);
                let ratio = self.get_f32(&["ratio"], 3.0);
                let attack_ms = self.get_f32(&["attack_ms", "attack"], 10.0);
                let release_ms = self.get_f32(&["release_ms", "release"], 100.0);
                let makeup_gain = self.get_f32(&["makeup_gain", "makeup_gain_db", "makeup"], 2.0);

                let mut comp = vocal_compressor(sample_rate);
                comp.set_params(
                    sample_rate,
                    threshold_db,
                    ratio,
                    attack_ms,
                    release_ms,
                    makeup_gain,
                );
                Box::new(comp)
            }

            "vocaldeesser" | "vocal_de_esser" | "deesser" | "de_esser" => {
                let freq = self.get_f32(&["frequency", "freq"], 6500.0);
                let threshold_db = self.get_f32(&["threshold_db", "threshold"], -20.0);
                let ratio = self.get_f32(&["ratio"], 4.0);
                let attack_ms = self.get_f32(&["attack_ms", "attack"], 1.0);
                let release_ms = self.get_f32(&["release_ms", "release"], 50.0);
                let mode_str = self.get_str(&["mode"], "SplitBand");
                let mode = if mode_str.eq_ignore_ascii_case("wideband")
                    || mode_str.eq_ignore_ascii_case("wide")
                {
                    DeEsserMode::WideBand
                } else {
                    DeEsserMode::SplitBand
                };

                let mut de_esser = VocalDeEsser::new(sample_rate, freq, threshold_db, ratio);
                de_esser.set_attack_ms(attack_ms);
                de_esser.set_release_ms(release_ms);
                de_esser.set_mode(mode);
                Box::new(de_esser)
            }

            "biquadfilter" | "biquad" | "filter" | "peakingeq" | "eq" => {
                let type_str = self.get_str(&["filter_type", "type"], "Peaking");
                let filter_type = parse_filter_type(type_str);
                let freq = self.get_f32(&["freq", "frequency", "cutoff_hz"], 1000.0);
                let q = self.get_f32(&["q", "resonance"], std::f32::consts::FRAC_1_SQRT_2);
                let gain_db = self.get_f32(&["gain_db", "gain"], 0.0);

                Box::new(BiquadFilter::new(
                    filter_type,
                    sample_rate,
                    freq,
                    q,
                    gain_db,
                ))
            }

            _ => return Err(format!("Unknown effect type: '{}'", self.effect_type)),
        };

        effect.set_enabled(!self.bypass);
        Ok(effect)
    }
}

/// Helper to parse cabinet profile name string into `CabinetProfile` enum.
fn parse_cabinet_profile(name: &str) -> CabinetProfile {
    let s = name.to_ascii_lowercase();
    if s.contains("twin") || s.contains("fender") {
        CabinetProfile::FenderTwin
    } else if s.contains("1960") || s.contains("marshall") {
        CabinetProfile::Marshall1960
    } else if s.contains("acoustic") {
        CabinetProfile::AcousticSim
    } else {
        CabinetProfile::CelestionV30
    }
}

/// Helper to parse filter type string into `FilterType` enum.
fn parse_filter_type(name: &str) -> FilterType {
    let s = name.to_ascii_lowercase();
    if s.contains("highpass") || s.contains("high_pass") || s.contains("hpf") {
        FilterType::HighPass
    } else if s.contains("lowpass") || s.contains("low_pass") || s.contains("lpf") {
        FilterType::LowPass
    } else if s.contains("highshelf") || s.contains("high_shelf") {
        FilterType::HighShelf
    } else if s.contains("lowshelf") || s.contains("low_shelf") {
        FilterType::LowShelf
    } else if s.contains("bandpass") || s.contains("band_pass") || s.contains("bpf") {
        FilterType::BandPass
    } else if s.contains("notch") {
        FilterType::Notch
    } else {
        FilterType::Peaking
    }
}
