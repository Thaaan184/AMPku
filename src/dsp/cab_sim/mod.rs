//! Cabinet impulse response (IR) simulation subsystem.
//!
//! Features zero-latency direct FIR convolution and 4 factory-calibrated
//! speaker cabinet profiles with zero heap allocations during processing.

pub mod factory_irs;
pub mod ir_engine;

pub use factory_irs::{generate_factory_ir, IR_LENGTH};
pub use ir_engine::{CabinetProfile, CabinetSim};
