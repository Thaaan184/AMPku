//! AMPku Studio GUI subsystem built on `egui`.
//!
//! Provides modular rack effect cards, dual channel strip mixers, dark studio theme tokens,
//! and real-time DSP control components.

pub mod channel_strip;
pub mod effect_card;
pub mod theme;

pub use channel_strip::{render_channel_strip, render_dual_channel_strips, AvailableEffect};
pub use effect_card::{render_effect_card, render_effect_card_indexed, EffectCardAction};
pub use theme::{
    apply_studio_theme, channel_strip_frame, effect_card_frame, meter_container_frame,
    COLOR_ACCENT_BLUE, COLOR_ALERT_RED, COLOR_BG, COLOR_CARD,
};
