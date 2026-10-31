//! Preset management, JSON serialization, and built-in factory tone configurations.

pub mod defaults;
pub mod manager;
pub mod schema;

pub use defaults::{
    acoustic_chime, clean_ambient, clear_vocal, get_factory_preset, get_factory_presets,
    modern_lead, texas_blues, FACTORY_PRESET_NAMES,
};
pub use manager::{PresetError, PresetManager};
pub use schema::{ChannelPreset, EffectPreset, MasterPreset, PresetData};
