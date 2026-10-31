//! Adaptive clock drift circular buffer with proportional-integral (PI) drift correction.
//!
//! Absorbs crystal oscillator frequency discrepancies between independent hardware
//! audio devices (e.g., USB interface input vs. motherboard DAC output) using
//! sub-sample Hermite cubic interpolation to avoid buffer underruns, stalls, and audible glitches.

use std::sync::{Arc, Mutex};

use crate::core::frame::AudioFrame;

/// Default proportional gain for drift controller.
pub const DEFAULT_KP: f32 = 0.0005;
/// Default integral gain for drift controller.
pub const DEFAULT_KI: f32 = 0.00005;
/// Minimum resampling ratio clamp (~-5% speed).
pub const DEFAULT_MIN_RATIO: f32 = 0.95;
/// Maximum resampling ratio clamp (~+5% speed).
pub const DEFAULT_MAX_RATIO: f32 = 1.05;

/// Thread-safe shared handle to a [`ClockDriftBuffer`].
pub type SharedClockDriftBuffer = Arc<Mutex<ClockDriftBuffer>>;

/// 4-point Hermite cubic spline interpolation.
///
/// Preserves audio fidelity with continuous first derivatives across sample boundaries.
#[inline]
pub fn hermite_interp(y_m1: f32, y0: f32, y1: f32, y2: f32, t: f32) -> f32 {
    let c0 = y0;
    let c1 = 0.5 * (y1 - y_m1);
    let c2 = y_m1 - 2.5 * y0 + 2.0 * y1 - 0.5 * y2;
    let c3 = 0.5 * (y2 - y_m1) + 1.5 * (y0 - y1);
    ((c3 * t + c2) * t + c1) * t + c0
}

/// Adaptive circular buffer with Proportional-Integral (PI) drift correction
/// and Hermite cubic interpolation.
#[derive(Debug, Clone)]
pub struct ClockDriftBuffer {
    buffer: Vec<AudioFrame>,
    capacity: usize,
    total_written: u64,
    total_read: f64,
    target_fill: f32,
    target_set: bool,
    kp: f32,
    ki: f32,
    integral: f32,
    drift_ratio: f32,
    min_ratio: f32,
    max_ratio: f32,
    underflow_count: u64,
    overflow_count: u64,
}

impl ClockDriftBuffer {
    /// Create a new circular drift buffer with a specified capacity in frames.
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(16);
        Self {
            buffer: vec![AudioFrame::ZERO; cap],
            capacity: cap,
            total_written: 0,
            total_read: 0.0,
            target_fill: (cap / 2) as f32,
            target_set: false,
            kp: DEFAULT_KP,
            ki: DEFAULT_KI,
            integral: 0.0,
            drift_ratio: 1.0,
            min_ratio: DEFAULT_MIN_RATIO,
            max_ratio: DEFAULT_MAX_RATIO,
            underflow_count: 0,
            overflow_count: 0,
        }
    }

    /// Create a new buffer with an explicit target fill level.
    pub fn with_target(capacity: usize, target_fill: f32) -> Self {
        let mut buf = Self::new(capacity);
        buf.set_target_fill(target_fill);
        buf
    }

    /// Create a thread-safe shared handle (`Arc<Mutex<ClockDriftBuffer>>`).
    pub fn shared(capacity: usize) -> SharedClockDriftBuffer {
        Arc::new(Mutex::new(Self::new(capacity)))
    }

    /// Convert this buffer into a thread-safe shared handle.
    pub fn into_shared(self) -> SharedClockDriftBuffer {
        Arc::new(Mutex::new(self))
    }

    /// Total buffer capacity in audio frames.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Current buffer occupancy in integer frames.
    #[inline]
    pub fn len(&self) -> usize {
        let occ = self.total_written as f64 - self.total_read;
        if occ <= 0.0 {
            0
        } else {
            (occ.floor() as usize).min(self.capacity)
        }
    }

    /// Return true if buffer contains zero readable frames.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return true if buffer occupancy has reached capacity.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }

    /// Number of available frames to read.
    #[inline]
    pub fn available_read(&self) -> usize {
        self.len()
    }

    /// Number of available frames to write before capacity is exceeded.
    #[inline]
    pub fn available_write(&self) -> usize {
        self.capacity.saturating_sub(self.len())
    }

    /// Target occupancy in frames for the PI drift controller.
    #[inline]
    pub fn target_fill(&self) -> f32 {
        self.target_fill
    }

    /// Set target occupancy in frames for the PI drift controller.
    pub fn set_target_fill(&mut self, target: f32) {
        self.target_fill = target.clamp(1.0, (self.capacity - 1) as f32);
        self.target_set = true;
    }

    /// Current adaptive playback/resampling speed ratio calculated by PI controller.
    #[inline]
    pub fn drift_ratio(&self) -> f32 {
        self.drift_ratio
    }

    /// Current PI controller error (`occupancy - target_fill`).
    #[inline]
    pub fn error(&self) -> f32 {
        let occ = (self.total_written as f64 - self.total_read).max(0.0) as f32;
        occ - self.target_fill
    }

    /// Current accumulated PI integral term.
    #[inline]
    pub fn integral(&self) -> f32 {
        self.integral
    }

    /// Get current PI controller gain constants `(Kp, Ki)`.
    #[inline]
    pub fn pi_constants(&self) -> (f32, f32) {
        (self.kp, self.ki)
    }

    /// Set PI controller gain constants `Kp` (proportional) and `Ki` (integral).
    pub fn set_pi_constants(&mut self, kp: f32, ki: f32) {
        self.kp = kp.max(0.0);
        self.ki = ki.max(0.0);
    }

    /// Set drift ratio clamping boundaries.
    pub fn set_ratio_limits(&mut self, min_ratio: f32, max_ratio: f32) {
        self.min_ratio = min_ratio.clamp(0.5, 1.0);
        self.max_ratio = max_ratio.clamp(1.0, 1.5);
    }

    /// Total number of frame underflow events (read requested when buffer starved).
    #[inline]
    pub fn underflow_count(&self) -> u64 {
        self.underflow_count
    }

    /// Total number of frame overflow events (write exceeded capacity, dropping oldest).
    #[inline]
    pub fn overflow_count(&self) -> u64 {
        self.overflow_count
    }

    /// Reset buffer state, pointers, and PI controller history.
    pub fn reset(&mut self) {
        self.buffer.fill(AudioFrame::ZERO);
        self.total_written = 0;
        self.total_read = 0.0;
        self.target_set = false;
        self.integral = 0.0;
        self.drift_ratio = 1.0;
        self.underflow_count = 0;
        self.overflow_count = 0;
    }

    /// Write a single audio frame into the circular buffer.
    ///
    /// Returns `true` if written normally, or `false` if buffer overflowed and oldest frame was dropped.
    pub fn write(&mut self, frame: AudioFrame) -> bool {
        let sanitized = frame.sanitize();
        let idx = (self.total_written as usize) % self.capacity;
        self.buffer[idx] = sanitized;
        self.total_written += 1;

        let occupancy = self.total_written as f64 - self.total_read;
        if occupancy > self.capacity as f64 {
            self.total_read = self.total_written as f64 - self.capacity as f64;
            self.overflow_count += 1;
            false
        } else {
            true
        }
    }

    /// Write a block of audio frames into the buffer. Returns number of frames written.
    pub fn write_block(&mut self, frames: &[AudioFrame]) -> usize {
        for &f in frames {
            self.write(f);
        }
        frames.len()
    }

    /// Read an in-place block of audio frames, applying PI drift compensation and Hermite interpolation.
    ///
    /// Missing frames on underflow are padded with [`AudioFrame::ZERO`]. Returns number of frames written to `output`.
    pub fn read(&mut self, output: &mut [AudioFrame]) -> usize {
        let count = output.len();
        if count == 0 {
            return 0;
        }

        let occupancy = (self.total_written as f64 - self.total_read).max(0.0);

        // Auto-lock target fill to initial occupancy on first read if not explicitly set
        if !self.target_set && self.total_written > 0 {
            let occ_f32 = occupancy as f32;
            self.target_fill = occ_f32.clamp(1.0, (self.capacity - 1) as f32);
            self.target_set = true;
        }

        // Update PI drift controller
        let err = (occupancy as f32) - self.target_fill;
        let p_term = self.kp * err;
        self.integral = (self.integral + self.ki * err).clamp(-0.05, 0.05);
        self.drift_ratio = (1.0 + p_term + self.integral).clamp(self.min_ratio, self.max_ratio);

        let r = self.drift_ratio as f64;
        let cap = self.capacity;

        for out_frame in output.iter_mut() {
            let available = self.total_written as f64 - self.total_read;
            if available < 1.0 {
                *out_frame = AudioFrame::ZERO;
                self.underflow_count += 1;
            } else {
                let idx_floor = self.total_read.floor();
                let idx_int = idx_floor as usize;
                let t = (self.total_read - idx_floor) as f32;

                let idx_0 = idx_int % cap;
                let idx_1 = (idx_int + 1) % cap;

                let (left, right) = if t <= 1e-6 {
                    let f = self.buffer[idx_0];
                    (f.left, f.right)
                } else if available < 2.0 {
                    // Linear fallback when near buffer edge
                    let f0 = self.buffer[idx_0];
                    let f1 = self.buffer[idx_1];
                    (
                        f0.left + t * (f1.left - f0.left),
                        f0.right + t * (f1.right - f0.right),
                    )
                } else {
                    // 4-point Hermite cubic interpolation
                    let idx_m1 = ((idx_int + cap).wrapping_sub(1)) % cap;
                    let idx_2 = (idx_int + 2) % cap;

                    let f_m1 = self.buffer[idx_m1];
                    let f_0 = self.buffer[idx_0];
                    let f_1 = self.buffer[idx_1];
                    let f_2 = self.buffer[idx_2];

                    (
                        hermite_interp(f_m1.left, f_0.left, f_1.left, f_2.left, t),
                        hermite_interp(f_m1.right, f_0.right, f_1.right, f_2.right, t),
                    )
                };

                *out_frame = AudioFrame::new(left, right).sanitize();
                self.total_read += r;
            }
        }

        count
    }

    /// Read a fixed count of audio frames as a `Vec<AudioFrame>`.
    ///
    /// Pads underflowing frames with [`AudioFrame::ZERO`]. Guaranteed to return `count` frames.
    pub fn read_block(&mut self, count: usize) -> Vec<AudioFrame> {
        let mut block = vec![AudioFrame::ZERO; count];
        self.read(&mut block);
        block
    }
}
