//! AMPku Studio GUI subsystem built on `egui`.
//!
//! Provides modular rack effect cards, dual channel strip mixers, dark studio theme tokens,
//! and real-time DSP control components.

pub mod app;
pub mod channel_strip;
pub mod effect_card;
pub mod meter;
pub mod theme;
pub mod transport;

pub use app::AmpkuApp;
pub use channel_strip::{render_channel_strip, render_dual_channel_strips, AvailableEffect};
pub use effect_card::{render_effect_card, render_effect_card_indexed, EffectCardAction};
pub use meter::{
    format_db, render_gain_reduction_meter, render_labeled_stereo_vu_meter, render_stereo_vu_meter,
    StereoVuMeter, VuMeterConfig,
};
pub use theme::{
    apply_studio_theme, channel_strip_frame, effect_card_frame, meter_container_frame,
    COLOR_ACCENT_BLUE, COLOR_ALERT_RED, COLOR_BG, COLOR_CARD,
};
pub use transport::{
    format_duration, format_latency, render_latency_badge, render_master_sum_bar, render_top_bar,
    TopBarState, TransportAction, BUFFER_SIZES,
};
