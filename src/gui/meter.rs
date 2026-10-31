//! Segmented stereo LED VU meter and limiter gain reduction display for AMPku Studio.
//!
//! Provides hardware-accurate stereo level metering with:
//! - Segmented 3-zone LED bars (Green: -60 dB to -12 dB, Yellow: -12 dB to 0 dB, Red: 0 dB to +3 dB)
//! - Peak hold indicators and RMS level body
//! - Latched digital clipping LED indicator (clickable to reset)
//! - Precision numeric dB readout (-60 dB to +3 dB)
//! - Brickwall limiter gain reduction (GR) meter widget

use egui::{pos2, vec2, Color32, Painter, Rect, Response, Sense, Ui};

use crate::engine::channel::ChannelMeter;
use crate::gui::theme::{
    meter_container_frame, COLOR_ALERT_RED, COLOR_METER_BG, COLOR_METER_GREEN, COLOR_METER_RED,
    COLOR_METER_YELLOW, COLOR_SOLO_AMBER, COLOR_TEXT_DIM, COLOR_TEXT_MUTED,
};

/// Minimum decibel floor displayed on the meter (-60 dBFS).
pub const MIN_METER_DB: f32 = -60.0;
/// Maximum decibel ceiling displayed on the meter (+3 dBFS).
pub const MAX_METER_DB: f32 = 3.0;
/// Decibel threshold where yellow warning segments begin (-12 dBFS).
pub const YELLOW_THRESHOLD_DB: f32 = -12.0;
/// Decibel threshold where red overload segments begin (0 dBFS).
pub const RED_THRESHOLD_DB: f32 = 0.0;
/// Default number of LED segments per channel bar.
pub const DEFAULT_NUM_SEGMENTS: usize = 24;

/// Dark unlit green LED segment background color.
pub const COLOR_METER_GREEN_DIM: Color32 = Color32::from_rgb(0x0d, 0x22, 0x14);
/// Dark unlit yellow LED segment background color.
pub const COLOR_METER_YELLOW_DIM: Color32 = Color32::from_rgb(0x24, 0x1d, 0x0a);
/// Dark unlit red LED segment background color.
pub const COLOR_METER_RED_DIM: Color32 = Color32::from_rgb(0x28, 0x10, 0x10);

/// Configuration parameters for the segmented stereo VU meter widget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VuMeterConfig {
    /// Minimum decibel level (default: -60.0 dB).
    pub min_db: f32,
    /// Maximum decibel level (default: +3.0 dB).
    pub max_db: f32,
    /// Yellow caution zone threshold (default: -12.0 dB).
    pub yellow_db: f32,
    /// Red clip warning zone threshold (default: 0.0 dB).
    pub red_db: f32,
    /// Number of discrete LED segments per column (default: 24).
    pub num_segments: usize,
    /// Whether to display the digital clip indicator LED at the top (default: true).
    pub show_clip: bool,
    /// Whether to display numeric peak dB readout text at the bottom (default: true).
    pub show_numeric: bool,
    /// Gap between adjacent LED segments in pixels (default: 1.5 px).
    pub segment_gap: f32,
}

impl Default for VuMeterConfig {
    fn default() -> Self {
        Self {
            min_db: MIN_METER_DB,
            max_db: MAX_METER_DB,
            yellow_db: YELLOW_THRESHOLD_DB,
            red_db: RED_THRESHOLD_DB,
            num_segments: DEFAULT_NUM_SEGMENTS,
            show_clip: true,
            show_numeric: true,
            segment_gap: 1.5,
        }
    }
}

/// Convert a decibel value into formatted studio readout text.
///
/// Returns `"-inf dB"` if `db <= -59.5`, `"+X.X dB"` if positive, or `"X.X dB"` otherwise.
pub fn format_db(db: f32) -> String {
    if !db.is_finite() || db <= -59.5 {
        "-inf dB".to_string()
    } else if db >= 0.05 {
        format!("+{:.1} dB", db)
    } else {
        format!("{:.1} dB", db)
    }
}

/// Convert decibels to a normalized scalar `[0.0, 1.0]` along `[min_db, max_db]`.
#[inline]
pub fn db_to_normalized(db: f32, min_db: f32, max_db: f32) -> f32 {
    if !db.is_finite() || db <= min_db {
        0.0
    } else if db >= max_db {
        1.0
    } else {
        (db - min_db) / (max_db - min_db)
    }
}

/// Segmented stereo LED VU meter widget.
#[derive(Debug, Clone)]
pub struct StereoVuMeter<'a> {
    config: VuMeterConfig,
    size: egui::Vec2,
    label: Option<&'a str>,
}

impl<'a> Default for StereoVuMeter<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> StereoVuMeter<'a> {
    /// Create a new `StereoVuMeter` with default configuration and standard dimensions (36x150).
    pub fn new() -> Self {
        Self {
            config: VuMeterConfig::default(),
            size: vec2(36.0, 150.0),
            label: None,
        }
    }

    /// Set custom dimensions for the meter widget.
    pub fn with_size(mut self, size: egui::Vec2) -> Self {
        self.size = size;
        self
    }

    /// Set custom configuration parameters.
    pub fn with_config(mut self, config: VuMeterConfig) -> Self {
        self.config = config;
        self
    }

    /// Set an optional channel label (e.g. "MASTER", "CH 1").
    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Render the stereo VU meter connected to a `ChannelMeter`.
    ///
    /// Clicking the widget resets the latched digital clipping indicator.
    pub fn show(&self, ui: &mut Ui, meter: &ChannelMeter) -> Response {
        let (peak_l, peak_r) = meter.peak_db();
        let (rms_l, rms_r) = meter.rms_db();
        let clipped = meter.is_clipped();

        let (response, reset_clip_requested) =
            self.show_raw(ui, peak_l, peak_r, rms_l, rms_r, clipped);

        if reset_clip_requested {
            meter.reset_clip();
        }

        response
    }

    /// Render the stereo VU meter from raw peak/rms decibel values and clip state.
    ///
    /// Returns `(egui::Response, clip_reset_clicked)`.
    pub fn show_raw(
        &self,
        ui: &mut Ui,
        peak_l_db: f32,
        peak_r_db: f32,
        rms_l_db: f32,
        rms_r_db: f32,
        clipped: bool,
    ) -> (Response, bool) {
        let mut clip_reset_clicked = false;

        let response = meter_container_frame()
            .show(ui, |ui| {
                if let Some(lbl) = self.label {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(lbl)
                                .size(9.5)
                                .strong()
                                .color(COLOR_TEXT_MUTED),
                        );
                    });
                    ui.add_space(2.0);
                }

                let (rect, response) = ui.allocate_exact_size(self.size, Sense::click());

                if response.clicked() {
                    clip_reset_clicked = true;
                }

                if ui.is_rect_visible(rect) {
                    let painter = ui.painter();
                    self.paint_meter(
                        painter,
                        rect,
                        (peak_l_db, peak_r_db),
                        (rms_l_db, rms_r_db),
                        clipped,
                    );
                }

                response
            })
            .inner;

        (response, clip_reset_clicked)
    }

    /// Paint the segmented LED bars, clipping indicator, and numeric readout.
    fn paint_meter(
        &self,
        painter: &Painter,
        rect: Rect,
        peak_db: (f32, f32),
        rms_db: (f32, f32),
        clipped: bool,
    ) {
        let (peak_l_db, peak_r_db) = peak_db;
        let (rms_l_db, rms_r_db) = rms_db;
        // Dark container background
        painter.rect_filled(rect, 2.0, COLOR_METER_BG);

        let mut current_y = rect.min.y + 2.0;

        // 1. Digital Clip Indicator LED (Height: 12.0 px)
        if self.config.show_clip {
            let clip_height = 12.0;
            let clip_rect = Rect::from_min_size(
                pos2(rect.min.x + 2.0, current_y),
                vec2(rect.width() - 4.0, clip_height),
            );

            let clip_fill = if clipped {
                COLOR_ALERT_RED
            } else {
                Color32::from_rgb(0x28, 0x10, 0x10)
            };
            let clip_text_color = if clipped {
                Color32::WHITE
            } else {
                Color32::from_rgb(0x5a, 0x22, 0x22)
            };

            painter.rect_filled(clip_rect, 2.0, clip_fill);
            painter.text(
                clip_rect.center(),
                egui::Align2::CENTER_CENTER,
                "CLIP",
                egui::FontId::new(7.5, egui::FontFamily::Monospace),
                clip_text_color,
            );

            current_y += clip_height + 4.0;
        }

        // 2. Numeric Readout at bottom (Height: 14.0 px)
        let numeric_height = if self.config.show_numeric { 14.0 } else { 0.0 };
        let bar_bottom_y = rect.max.y - 2.0 - numeric_height;
        let bar_height = (bar_bottom_y - current_y).max(10.0);

        // 3. Dual LED Bars (Left & Right channels)
        let bar_gap = 2.0;
        let total_bar_width = rect.width() - 6.0;
        let bar_width = ((total_bar_width - bar_gap) * 0.5).max(4.0);

        let left_x = rect.min.x + 3.0;
        let right_x = left_x + bar_width + bar_gap;

        let left_bar_rect =
            Rect::from_min_size(pos2(left_x, current_y), vec2(bar_width, bar_height));
        let right_bar_rect =
            Rect::from_min_size(pos2(right_x, current_y), vec2(bar_width, bar_height));

        self.paint_segmented_channel_bar(painter, left_bar_rect, peak_l_db, rms_l_db);

        self.paint_segmented_channel_bar(painter, right_bar_rect, peak_r_db, rms_r_db);

        // 4. Numeric dB Readout Text
        if self.config.show_numeric {
            let max_peak = peak_l_db.max(peak_r_db);
            let readout_text = format_db(max_peak);
            let readout_pos = pos2(rect.center().x, rect.max.y - 8.0);

            let text_color = if max_peak >= self.config.red_db {
                COLOR_ALERT_RED
            } else if max_peak >= self.config.yellow_db {
                COLOR_METER_YELLOW
            } else {
                COLOR_TEXT_DIM
            };

            painter.text(
                readout_pos,
                egui::Align2::CENTER_CENTER,
                readout_text,
                egui::FontId::new(8.5, egui::FontFamily::Monospace),
                text_color,
            );
        }
    }

    /// Paint a single vertical column of segmented LEDs with RMS level body and peak hold dot.
    fn paint_segmented_channel_bar(
        &self,
        painter: &Painter,
        rect: Rect,
        peak_db: f32,
        rms_db: f32,
    ) {
        let n = self.config.num_segments.max(4);
        let gap = self.config.segment_gap;
        let total_gaps = (n - 1) as f32 * gap;
        let seg_height = ((rect.height() - total_gaps) / n as f32).max(1.0);

        let min_db = self.config.min_db;
        let max_db = self.config.max_db;
        let yellow_db = self.config.yellow_db;
        let red_db = self.config.red_db;

        // Determine which segment index peak hold lands in (0 = lowest dB, n-1 = highest dB)
        let peak_norm = db_to_normalized(peak_db, min_db, max_db);
        let peak_segment_idx = ((peak_norm * n as f32).floor() as usize).min(n.saturating_sub(1));

        for i in 0..n {
            // i = 0 is bottom segment (lowest dB), i = n - 1 is top segment (highest dB)
            let seg_min_db = min_db + (max_db - min_db) * (i as f32 / n as f32);
            let seg_max_db = min_db + (max_db - min_db) * ((i + 1) as f32 / n as f32);

            // Compute Y coordinates (top of rect is highest dB / index n-1)
            let seg_top_y = rect.max.y - (i + 1) as f32 * seg_height - i as f32 * gap;
            let seg_rect =
                Rect::from_min_size(pos2(rect.min.x, seg_top_y), vec2(rect.width(), seg_height));

            // Determine LED color zone
            let (lit_color, dim_color) = if seg_max_db > red_db {
                (COLOR_METER_RED, COLOR_METER_RED_DIM)
            } else if seg_max_db > yellow_db {
                (COLOR_METER_YELLOW, COLOR_METER_YELLOW_DIM)
            } else {
                (COLOR_METER_GREEN, COLOR_METER_GREEN_DIM)
            };

            // Is this segment lit by RMS?
            let is_lit = rms_db >= seg_min_db;

            // Is this segment the peak hold?
            let is_peak_hold = i == peak_segment_idx && peak_db > min_db;

            let fill_color = if is_peak_hold {
                Color32::WHITE
            } else if is_lit {
                lit_color
            } else {
                dim_color
            };

            painter.rect_filled(seg_rect, 0.5, fill_color);
        }
    }
}

/// Render a standard segmented stereo VU meter for a `ChannelMeter`.
pub fn render_stereo_vu_meter(ui: &mut Ui, meter: &ChannelMeter, size: egui::Vec2) -> Response {
    StereoVuMeter::new().with_size(size).show(ui, meter)
}

/// Render a labeled segmented stereo VU meter for a `ChannelMeter`.
pub fn render_labeled_stereo_vu_meter(
    ui: &mut Ui,
    meter: &ChannelMeter,
    label: &str,
    size: egui::Vec2,
) -> Response {
    StereoVuMeter::new()
        .with_label(label)
        .with_size(size)
        .show(ui, meter)
}

/// Render a brickwall limiter gain reduction (GR) meter.
///
/// Displays negative gain reduction from 0 dB down to -12 dB with precision numeric readout.
pub fn render_gain_reduction_meter(ui: &mut Ui, gr_db: f32, width: f32, height: f32) {
    meter_container_frame().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("GR")
                    .size(9.0)
                    .strong()
                    .color(COLOR_SOLO_AMBER),
            );

            let desired_size = vec2(width, height);
            let (rect, _response) = ui.allocate_exact_size(desired_size, Sense::hover());

            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                painter.rect_filled(rect, 1.5, COLOR_METER_BG);

                // GR dB is negative or 0.0 (e.g. -3.0 dB)
                // Normalize 0.0 dB to -12.0 dB
                let gr_abs = gr_db.abs();
                let gr_norm = (gr_abs / 12.0).clamp(0.0, 1.0);

                if gr_norm > 0.01 {
                    // Draw downward amber/red reduction bar from top of meter
                    let fill_height = (rect.height() * gr_norm).max(1.0);
                    let fill_rect = Rect::from_min_size(
                        pos2(rect.min.x + 1.0, rect.min.y + 1.0),
                        vec2(rect.width() - 2.0, fill_height),
                    );

                    let gr_color = if gr_abs > 6.0 {
                        COLOR_ALERT_RED
                    } else if gr_abs > 3.0 {
                        COLOR_SOLO_AMBER
                    } else {
                        COLOR_METER_YELLOW
                    };

                    painter.rect_filled(fill_rect, 1.0, gr_color);
                }

                // Numeric readout at bottom
                let readout = if gr_abs < 0.1 {
                    "0.0".to_string()
                } else {
                    format!("-{:.1}", gr_abs)
                };

                painter.text(
                    pos2(rect.center().x, rect.max.y - 5.0),
                    egui::Align2::CENTER_CENTER,
                    readout,
                    egui::FontId::new(8.0, egui::FontFamily::Monospace),
                    if gr_abs > 0.1 {
                        COLOR_SOLO_AMBER
                    } else {
                        COLOR_TEXT_DIM
                    },
                );
            }
        });
    });
}
