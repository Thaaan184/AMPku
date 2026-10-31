//! Audio backend, hardware streaming, and background WAV recording.

pub mod recorder;

pub use recorder::{i24_to_sample, sample_to_i24, RecorderError, WavRecorder};
