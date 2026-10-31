//! DSP processing engine and 9 custom guitar effects.

pub mod amp;
pub mod chain;
pub mod delay;
pub mod distortion;
pub mod eq;
pub mod gain;
pub mod gate;
pub mod limiter;
pub mod overdrive;
pub mod reverb;
pub mod traits;

pub use amp::{AmpSimulator, AmpVoicing};
pub use chain::{
    to_db, DSPChain, AMP_SIM_IDX, DELAY_IDX, DISTORTION_IDX, EQ_IDX, INPUT_GAIN_IDX, LIMITER_IDX,
    NOISE_GATE_IDX, OVERDRIVE_IDX, REVERB_IDX,
};
pub use delay::Delay;
pub use distortion::Distortion;
pub use eq::Equalizer;
pub use gain::InputGain;
pub use gate::NoiseGate;
pub use limiter::Limiter;
pub use overdrive::Overdrive;
pub use reverb::Reverb;
pub use traits::{flush_denormal, AudioEffect};
