//! Dark studio utility design tokens and custom frame styles for AMPku UI.
//!
//! Features studio utility color palette:
//! - `#18181b` Zinc-900 studio background
//! - `#27272a` Zinc-800 modular effect card container
//! - `#38bdf8` Sky-400 studio electric accent blue
//! - `#ef4444` Red-500 clip alert and mute warning
//! - Custom frames for channel strips, modular cards, VU meters, and headers.

use egui::{Color32, FontFamily, FontId, Frame, Margin, Rounding, Stroke, TextStyle, Visuals};

// Studio Utility Color Palette Constants
pub const COLOR_BG: Color32 = Color32::from_rgb(0x18, 0x18, 0x1b);
pub const COLOR_CARD: Color32 = Color32::from_rgb(0x27, 0x27, 0x2a);
pub const COLOR_CARD_HEADER: Color32 = Color32::from_rgb(0x20, 0x20, 0x24);
pub const COLOR_CARD_HOVER: Color32 = Color32::from_rgb(0x32, 0x32, 0x36);
pub const COLOR_BORDER: Color32 = Color32::from_rgb(0x3f, 0x3f, 0x46);
pub const COLOR_BORDER_ACTIVE: Color32 = Color32::from_rgb(0x52, 0x52, 0x5b);
pub const COLOR_ACCENT_BLUE: Color32 = Color32::from_rgb(0x38, 0xbd, 0xf8);
pub const COLOR_ACCENT_BLUE_HOVER: Color32 = Color32::from_rgb(0x0e, 0xa5, 0xe9);
pub const COLOR_ALERT_RED: Color32 = Color32::from_rgb(0xef, 0x44, 0x44);
pub const COLOR_ALERT_RED_BG: Color32 = Color32::from_rgb(0x7f, 0x1d, 0x1d);
pub const COLOR_ACTIVE_GREEN: Color32 = Color32::from_rgb(0x22, 0xc5, 0x5e);
pub const COLOR_SOLO_AMBER: Color32 = Color32::from_rgb(0xf5, 0x9e, 0x0b);
pub const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(0xf4, 0xf4, 0xf5);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(0xa1, 0xa1, 0xaa);
pub const COLOR_TEXT_DIM: Color32 = Color32::from_rgb(0x71, 0x71, 0x7a);
pub const COLOR_METER_BG: Color32 = Color32::from_rgb(0x09, 0x09, 0x0b);
pub const COLOR_METER_GREEN: Color32 = Color32::from_rgb(0x22, 0xc5, 0x5e);
pub const COLOR_METER_YELLOW: Color32 = Color32::from_rgb(0xea, 0xb3, 0x08);
pub const COLOR_METER_RED: Color32 = Color32::from_rgb(0xef, 0x44, 0x44);

/// Custom frame for modular effect rack cards.
pub fn effect_card_frame(enabled: bool) -> Frame {
    let border_color = if enabled {
        COLOR_BORDER_ACTIVE
    } else {
        Color32::from_rgb(0x33, 0x33, 0x38)
    };
    Frame::none()
        .fill(COLOR_CARD)
        .stroke(Stroke::new(1.0_f32, border_color))
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::same(8.0))
        .outer_margin(Margin::symmetric(0.0, 4.0))
}

/// Custom frame for channel strip container column.
pub fn channel_strip_frame() -> Frame {
    Frame::none()
        .fill(Color32::from_rgb(0x1e, 0x1e, 0x22))
        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(12.0))
        .outer_margin(Margin::same(4.0))
}

/// Custom frame for level meters and VU displays.
pub fn meter_container_frame() -> Frame {
    Frame::none()
        .fill(COLOR_METER_BG)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x27, 0x27, 0x2a)))
        .rounding(Rounding::same(3.0))
        .inner_margin(Margin::symmetric(4.0, 4.0))
}

/// Custom frame for card headers.
pub fn header_frame() -> Frame {
    Frame::none()
        .fill(COLOR_CARD_HEADER)
        .rounding(Rounding {
            nw: 6.0,
            ne: 6.0,
            sw: 0.0,
            se: 0.0,
        })
        .inner_margin(Margin::symmetric(8.0, 6.0))
}

/// Frame for control panels or fader well.
pub fn well_frame() -> Frame {
    Frame::none()
        .fill(Color32::from_rgb(0x14, 0x14, 0x17))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x27, 0x27, 0x2a)))
        .rounding(Rounding::same(4.0))
        .inner_margin(Margin::same(6.0))
}

/// Configure the egui Context with the AMPku studio dark theme and typography.
pub fn apply_studio_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    // Dark studio base colors
    visuals.panel_fill = COLOR_BG;
    visuals.window_fill = COLOR_CARD;
    visuals.override_text_color = Some(COLOR_TEXT_PRIMARY);

    // Accent selection
    visuals.selection.bg_fill = COLOR_ACCENT_BLUE;
    visuals.selection.stroke = Stroke::new(1.0_f32, COLOR_ACCENT_BLUE_HOVER);

    // Widgets styling
    visuals.widgets.noninteractive.bg_fill = COLOR_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    visuals.widgets.noninteractive.rounding = Rounding::same(4.0);

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(0x2d, 0x2d, 0x31);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    visuals.widgets.inactive.rounding = Rounding::same(4.0);

    visuals.widgets.hovered.bg_fill = COLOR_CARD_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, COLOR_ACCENT_BLUE);
    visuals.widgets.hovered.rounding = Rounding::same(4.0);

    visuals.widgets.active.bg_fill = Color32::from_rgb(0x02, 0x84, 0xc7); // sky-600
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, COLOR_ACCENT_BLUE);
    visuals.widgets.active.rounding = Rounding::same(4.0);

    visuals.widgets.open.bg_fill = COLOR_CARD_HOVER;
    visuals.widgets.open.rounding = Rounding::same(4.0);

    ctx.set_visuals(visuals);

    // Typography hierarchy
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(17.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Name("Subheading".into()),
            FontId::new(14.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(12.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(12.0, FontFamily::Monospace),
        ),
        (
            TextStyle::Small,
            FontId::new(10.5, FontFamily::Proportional),
        ),
    ]
    .into();

    style.spacing.item_spacing = egui::vec2(6.0, 6.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.slider_width = 140.0;

    ctx.set_style(style);
}
