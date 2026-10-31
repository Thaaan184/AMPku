//! Audio backend, hardware streaming, adaptive clock drift management, and WAV recording.

pub mod clock_drift;
pub mod device;
pub mod engine;
pub mod recorder;

pub use clock_drift::{hermite_interp, ClockDriftBuffer, SharedClockDriftBuffer};
pub use device::{
    calculate_latency_ms, default_device, find_device, latency_badge, query_input_devices,
    query_output_devices, validate_buffer_size, validate_sample_rate, AudioDeviceInfo,
    AudioStreamConfig, DeviceError, DEFAULT_BUFFER_SIZE, DEFAULT_SAMPLE_RATE,
    SUPPORTED_BUFFER_SIZES, SUPPORTED_SAMPLE_RATES,
};
pub use engine::{AudioEngine, EngineMeters};
pub use recorder::{i24_to_sample, sample_to_i24, RecorderError, WavRecorder};
