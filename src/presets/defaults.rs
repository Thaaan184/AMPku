//! Built-in factory tone presets for AMPku.
//!
//! Provides five calibrated factory presets:
//! - `TexasBlues`: SRV-style tube screamer overdrive, Fender Twin 2x12 cab, spring reverb.
//! - `ModernLead`: Tight noise-gated high-gain distortion, Marshall 1960A 4x12 cab, ping-pong delay.
//! - `CleanAmbient`: Studio compressor, Fender Twin cab, dual modulated delay, expansive hall reverb.
//! - `AcousticChime`: High-shelf acoustic sparkle EQ, Acoustic Sim IR cab, gentle room reverb.
//! - `ClearVocal`: Broadcast vocal strip with 80 Hz HPF, dynamic de-esser, compressor, presence EQ, and reverb.

use crate::presets::schema::{ChannelPreset, EffectPreset, MasterPreset, PresetData};

/// List of built-in factory preset names.
pub const FACTORY_PRESET_NAMES: [&str; 5] = [
    "TexasBlues",
    "ModernLead",
    "CleanAmbient",
    "AcousticChime",
    "ClearVocal",
];

/// Factory preset: Texas Blues (SRV-style overdrive & Fender Twin cab).
pub fn texas_blues() -> PresetData {
    PresetData::new(
        "TexasBlues",
        ChannelPreset::new(0.85, 0.0).with_effects(vec![
            EffectPreset::noise_gate(-55.0, 2.0, 60.0, false),
            EffectPreset::overdrive(8.5, 0.65, 1.1, false),
            EffectPreset::cab_sim("FenderTwin", 1.0, 0.0, false),
            EffectPreset::reverb(0.45, 0.50, 0.85, 0.22, false),
        ]),
        ChannelPreset::new(0.85, 0.0).with_effects(vec![
            EffectPreset::vocal_high_pass(80.0, false),
            EffectPreset::vocal_compressor(-18.0, 3.0, 10.0, 100.0, 2.0, false),
            EffectPreset::reverb(0.40, 0.40, 0.90, 0.18, false),
        ]),
    )
    .with_description(
        "Classic SRV-style Texas blues overdrive through a 2x12 Fender Twin cab \
         with warm spring room reverb and balanced vocal mic channel.",
    )
    .with_category("Blues")
    .with_master(MasterPreset::default())
}

/// Factory preset: Modern Lead (High-gain articulate lead with ping-pong delay).
pub fn modern_lead() -> PresetData {
    PresetData::new(
        "ModernLead",
        ChannelPreset::new(0.80, 0.0).with_effects(vec![
            EffectPreset::noise_gate(-48.0, 1.5, 35.0, false),
            EffectPreset::distortion(18.0, 0.55, 0.95, false),
            EffectPreset::cab_sim("Marshall1960", 1.0, 0.0, false),
            EffectPreset::delay(380.0, 0.35, 0.28, 0.40, true, 1.0, false),
            EffectPreset::reverb(0.70, 0.35, 1.0, 0.20, false),
        ]),
        ChannelPreset::new(0.85, 0.0).with_effects(vec![
            EffectPreset::vocal_high_pass(80.0, false),
            EffectPreset::vocal_de_esser(6500.0, -20.0, 4.0, 1.0, 40.0, "SplitBand", false),
            EffectPreset::vocal_compressor(-16.0, 3.5, 8.0, 90.0, 2.5, false),
        ]),
    )
    .with_description(
        "High-gain articulate modern lead tone with tight noise gating, \
         4x12 Marshall cab sim, stereophonic ping-pong delay, and hall reverb.",
    )
    .with_category("Rock")
    .with_master(MasterPreset::default())
}

/// Factory preset: Clean Ambient (Lush ethereal clean tone with spacious delay & reverb).
pub fn clean_ambient() -> PresetData {
    PresetData::new(
        "CleanAmbient",
        ChannelPreset::new(0.85, 0.0).with_effects(vec![
            EffectPreset::compressor(-22.0, 3.0, 15.0, 120.0, 2.0, false),
            EffectPreset::cab_sim("FenderTwin", 1.0, 0.0, false),
            EffectPreset::delay(460.0, 0.45, 0.35, 0.25, true, 1.0, false),
            EffectPreset::reverb(0.85, 0.20, 1.0, 0.45, false),
        ]),
        ChannelPreset::new(0.85, 0.0).with_effects(vec![
            EffectPreset::vocal_high_pass(80.0, false),
            EffectPreset::vocal_compressor(-18.0, 3.0, 10.0, 100.0, 2.0, false),
            EffectPreset::reverb(0.75, 0.30, 1.0, 0.30, false),
        ]),
    )
    .with_description(
        "Lush ethereal clean tone with studio compressor, twin cab warmth, \
         long modulated stereo ping-pong delay, and expansive hall reverb.",
    )
    .with_category("Ambient")
    .with_master(MasterPreset::default())
}

/// Factory preset: Acoustic Chime (Bright acoustic resonance with air high-shelf boost).
pub fn acoustic_chime() -> PresetData {
    PresetData::new(
        "AcousticChime",
        ChannelPreset::new(0.85, 0.0).with_effects(vec![
            EffectPreset::compressor(-18.0, 2.5, 20.0, 140.0, 1.5, false),
            EffectPreset::biquad(
                "HighShelf",
                4800.0,
                std::f32::consts::FRAC_1_SQRT_2,
                2.5,
                false,
            ),
            EffectPreset::cab_sim("AcousticSim", 1.0, 0.0, false),
            EffectPreset::reverb(0.50, 0.45, 0.90, 0.22, false),
        ]),
        ChannelPreset::new(0.90, 0.0).with_effects(vec![
            EffectPreset::vocal_high_pass(80.0, false),
            EffectPreset::vocal_de_esser(6800.0, -22.0, 3.5, 1.2, 50.0, "SplitBand", false),
            EffectPreset::vocal_compressor(-16.0, 2.8, 12.0, 110.0, 2.0, false),
            EffectPreset::reverb(0.55, 0.40, 0.90, 0.20, false),
        ]),
    )
    .with_description(
        "Sparkling acoustic guitar tone featuring body contour IR, gentle \
         dynamics smoothing, high-shelf air boost, and ambient space reverb.",
    )
    .with_category("Acoustic")
    .with_master(MasterPreset::default())
}

/// Factory preset: Clear Vocal (Studio broadcast-ready vocal chain & subtle backing guitar).
pub fn clear_vocal() -> PresetData {
    PresetData::new(
        "ClearVocal",
        ChannelPreset::new(0.75, 0.0).with_effects(vec![
            EffectPreset::noise_gate(-55.0, 3.0, 70.0, false),
            EffectPreset::compressor(-20.0, 2.5, 15.0, 120.0, 1.0, false),
            EffectPreset::cab_sim("CelestionV30", 0.9, 0.0, false),
            EffectPreset::reverb(0.40, 0.50, 0.80, 0.18, false),
        ]),
        ChannelPreset::new(0.95, 0.0).with_effects(vec![
            EffectPreset::vocal_high_pass(80.0, false),
            EffectPreset::vocal_de_esser(6200.0, -24.0, 4.5, 0.8, 35.0, "SplitBand", false),
            EffectPreset::vocal_compressor(-20.0, 3.5, 8.0, 85.0, 3.0, false),
            EffectPreset::biquad("Peaking", 3500.0, 1.2, 2.0, false),
            EffectPreset::reverb(0.55, 0.35, 0.95, 0.22, false),
        ]),
    )
    .with_description(
        "Broadcast and studio-grade vocal processing suite with dynamic \
         de-essing, opto compression, presence EQ, and subtle backing guitar.",
    )
    .with_category("Vocal")
    .with_master(MasterPreset::default())
}

/// Return all five factory presets as an allocated `Vec<PresetData>`.
pub fn get_factory_presets() -> Vec<PresetData> {
    vec![
        texas_blues(),
        modern_lead(),
        clean_ambient(),
        acoustic_chime(),
        clear_vocal(),
    ]
}

/// Lookup a built-in factory preset by name (case-insensitive).
pub fn get_factory_preset(name: &str) -> Option<PresetData> {
    let lower = name.trim().to_ascii_lowercase();
    match lower.as_str() {
        "texasblues" | "texas_blues" | "texas blues" => Some(texas_blues()),
        "modernlead" | "modern_lead" | "modern lead" => Some(modern_lead()),
        "cleanambient" | "clean_ambient" | "clean ambient" => Some(clean_ambient()),
        "acousticchime" | "acoustic_chime" | "acoustic chime" => Some(acoustic_chime()),
        "clearvocal" | "clear_vocal" | "clear vocal" => Some(clear_vocal()),
        _ => None,
    }
}
