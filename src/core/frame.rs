//! Stereo audio frame representation with arithmetic operations and sanitization.

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Stereo audio sample frame containing 32-bit floating point left and right channels.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct AudioFrame {
    pub left: f32,
    pub right: f32,
}

impl AudioFrame {
    /// Frame with zero amplitude on both channels.
    pub const ZERO: Self = Self {
        left: 0.0,
        right: 0.0,
    };

    /// Create a new stereo frame with individual left and right sample values.
    #[inline]
    pub const fn new(left: f32, right: f32) -> Self {
        Self { left, right }
    }

    /// Create a stereo frame from a mono source by duplicating the sample to both channels.
    #[inline]
    pub const fn from_mono(sample: f32) -> Self {
        Self {
            left: sample,
            right: sample,
        }
    }

    /// Downmix the stereo frame to a mono scalar sample `(left + right) * 0.5`.
    #[inline]
    pub fn to_mono(&self) -> f32 {
        (self.left + self.right) * 0.5
    }

    /// Clamp both channels within the range `[min, max]`.
    #[inline]
    pub fn clamp(self, min: f32, max: f32) -> Self {
        Self {
            left: self.left.clamp(min, max),
            right: self.right.clamp(min, max),
        }
    }

    /// Clamp both channels within the symmetric range `[-limit, limit]`.
    #[inline]
    pub fn clamp_symmetric(self, limit: f32) -> Self {
        let abs_limit = limit.abs();
        self.clamp(-abs_limit, abs_limit)
    }

    /// Replace NaN or infinite values with 0.0 to prevent pipeline contamination.
    #[inline]
    pub fn sanitize(self) -> Self {
        Self {
            left: if self.left.is_finite() {
                self.left
            } else {
                0.0
            },
            right: if self.right.is_finite() {
                self.right
            } else {
                0.0
            },
        }
    }

    /// Return true if both channel samples are finite numbers (not NaN, not Inf).
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.left.is_finite() && self.right.is_finite()
    }

    /// Return an AudioFrame with absolute values for both channels.
    #[inline]
    pub fn abs(self) -> Self {
        Self {
            left: self.left.abs(),
            right: self.right.abs(),
        }
    }

    /// Return the maximum absolute peak value across both channels.
    #[inline]
    pub fn peak(&self) -> f32 {
        self.left.abs().max(self.right.abs())
    }
}

impl Add for AudioFrame {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            left: self.left + rhs.left,
            right: self.right + rhs.right,
        }
    }
}

impl AddAssign for AudioFrame {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.left += rhs.left;
        self.right += rhs.right;
    }
}

impl Add<f32> for AudioFrame {
    type Output = Self;

    #[inline]
    fn add(self, rhs: f32) -> Self::Output {
        Self {
            left: self.left + rhs,
            right: self.right + rhs,
        }
    }
}

impl Add<AudioFrame> for f32 {
    type Output = AudioFrame;

    #[inline]
    fn add(self, rhs: AudioFrame) -> Self::Output {
        rhs + self
    }
}

impl AddAssign<f32> for AudioFrame {
    #[inline]
    fn add_assign(&mut self, rhs: f32) {
        self.left += rhs;
        self.right += rhs;
    }
}

impl Sub for AudioFrame {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            left: self.left - rhs.left,
            right: self.right - rhs.right,
        }
    }
}

impl SubAssign for AudioFrame {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.left -= rhs.left;
        self.right -= rhs.right;
    }
}

impl Sub<f32> for AudioFrame {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: f32) -> Self::Output {
        Self {
            left: self.left - rhs,
            right: self.right - rhs,
        }
    }
}

impl SubAssign<f32> for AudioFrame {
    #[inline]
    fn sub_assign(&mut self, rhs: f32) {
        self.left -= rhs;
        self.right -= rhs;
    }
}

impl Mul for AudioFrame {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            left: self.left * rhs.left,
            right: self.right * rhs.right,
        }
    }
}

impl MulAssign for AudioFrame {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.left *= rhs.left;
        self.right *= rhs.right;
    }
}

impl Mul<f32> for AudioFrame {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            left: self.left * rhs,
            right: self.right * rhs,
        }
    }
}

impl Mul<AudioFrame> for f32 {
    type Output = AudioFrame;

    #[inline]
    fn mul(self, rhs: AudioFrame) -> Self::Output {
        rhs * self
    }
}

impl MulAssign<f32> for AudioFrame {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        self.left *= rhs;
        self.right *= rhs;
    }
}

impl Div<f32> for AudioFrame {
    type Output = Self;

    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        Self {
            left: self.left / rhs,
            right: self.right / rhs,
        }
    }
}

impl DivAssign<f32> for AudioFrame {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        self.left /= rhs;
        self.right /= rhs;
    }
}

impl Neg for AudioFrame {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self {
            left: -self.left,
            right: -self.right,
        }
    }
}

impl From<f32> for AudioFrame {
    #[inline]
    fn from(value: f32) -> Self {
        Self::from_mono(value)
    }
}

impl From<(f32, f32)> for AudioFrame {
    #[inline]
    fn from((left, right): (f32, f32)) -> Self {
        Self::new(left, right)
    }
}

impl From<[f32; 2]> for AudioFrame {
    #[inline]
    fn from([left, right]: [f32; 2]) -> Self {
        Self::new(left, right)
    }
}

impl From<AudioFrame> for (f32, f32) {
    #[inline]
    fn from(frame: AudioFrame) -> Self {
        (frame.left, frame.right)
    }
}

impl From<AudioFrame> for [f32; 2] {
    #[inline]
    fn from(frame: AudioFrame) -> Self {
        [frame.left, frame.right]
    }
}
