//! Saturation and distortion effects (overdrive, distortion, fuzz).

pub mod distortion;
pub mod fuzz;
pub mod overdrive;

pub use distortion::Distortion;
pub use fuzz::Fuzz;
pub use overdrive::Overdrive;
