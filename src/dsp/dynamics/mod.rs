//! Dynamics processing modules (noise gate, compressor).

pub mod compressor;
pub mod noise_gate;

pub use compressor::Compressor;
pub use noise_gate::{GateState, NoiseGate};
