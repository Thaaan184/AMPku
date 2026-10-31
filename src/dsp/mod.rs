//! Digital Signal Processing (DSP) subsystem for AMPku.

pub mod cab_sim;
pub mod dynamics;
pub mod filters;
pub mod saturation;
pub mod time;
pub mod traits;
pub mod vocal;

pub use cab_sim::{CabinetProfile, CabinetSim};
pub use dynamics::{Compressor, GateState, NoiseGate};
pub use filters::biquad::{BiquadCoefficients, BiquadFilter, FilterType};
pub use saturation::{Distortion, Fuzz, Overdrive};
pub use time::{StereoDelay, StereoReverb};
pub use traits::AudioEffect;
pub use vocal::{vocal_compressor, vocal_high_pass, DeEsserMode, VocalDeEsser};
