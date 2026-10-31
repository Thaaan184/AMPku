//! Unified DSP trait definitions for audio processing nodes.

use crate::core::frame::AudioFrame;

/// Core trait implemented by all audio processing nodes and effects in AMPku.
///
/// Implementations must be real-time safe:
/// - Zero heap allocations (`malloc`/`free`) during processing.
/// - Zero system calls and zero blocking locks.
/// - Robust NaN and Inf handling.
pub trait AudioEffect: Send + 'static {
    /// Process a single stereo audio frame.
    fn process_frame(&mut self, frame: AudioFrame) -> AudioFrame;

    /// Process a block of stereo frames in-place.
    #[inline]
    fn process_block(&mut self, frames: &mut [AudioFrame]) {
        for frame in frames.iter_mut() {
            *frame = self.process_frame(*frame);
        }
    }

    /// Reset internal state buffers (e.g. filter histories, delay lines, envelope detectors).
    fn reset(&mut self);

    /// Return the display name of this effect node.
    fn name(&self) -> &'static str {
        "Audio Effect"
    }

    /// Return true if the effect is currently active (not bypassed).
    fn is_enabled(&self) -> bool {
        true
    }

    /// Enable or bypass the effect node.
    fn set_enabled(&mut self, _enabled: bool) {}

    /// Return reference as `&dyn std::any::Any` for dynamic inspection and GUI downcasting.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Return mutable reference as `&mut dyn std::any::Any` for dynamic inspection and GUI downcasting.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
