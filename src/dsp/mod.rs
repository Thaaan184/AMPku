//! Digital Signal Processing (DSP) subsystem for AMPku.

pub mod dynamics;
pub mod filters;
pub mod saturation;
pub mod traits;

pub use dynamics::{Compressor, GateState, NoiseGate};
pub use filters::biquad::{BiquadCoefficients, BiquadFilter, FilterType};
pub use saturation::{Distortion, Fuzz, Overdrive};
pub use traits::AudioEffect;
