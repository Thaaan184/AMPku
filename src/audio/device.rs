//! CPAL audio hardware device enumeration, stream configuration, and latency validation.

use cpal::traits::{DeviceTrait, HostTrait};

/// Supported low-latency buffer frame sizes.
pub const SUPPORTED_BUFFER_SIZES: &[u32] = &[64, 128, 256, 512, 1024];

/// Supported standard audio sample rates in Hz.
pub const SUPPORTED_SAMPLE_RATES: &[u32] = &[44100, 48000, 88200, 96000, 192000];

/// Default buffer size for ultra-low latency monitoring (64 frames = ~1.33 ms @ 48 kHz).
pub const DEFAULT_BUFFER_SIZE: u32 = 64;

/// Default sample rate in Hz.
pub const DEFAULT_SAMPLE_RATE: u32 = 48000;

/// Errors associated with hardware audio device discovery, configuration, and stream setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceError {
    /// Device with specified name was not found.
    DeviceNotFound(String),
    /// System has no default audio device configured.
    NoDefaultDevice(String),
    /// Hardware configuration is not supported by device driver.
    UnsupportedConfig(String),
    /// Requested buffer size is not in supported list (64, 128, 256, 512, 1024).
    InvalidBufferSize(u32),
    /// Requested sample rate is not in supported list (44100, 48000, 88200, etc.).
    InvalidSampleRate(u32),
    /// Failure during CPAL stream creation.
    StreamBuildError(String),
    /// Failure starting or playing CPAL stream.
    StreamPlayError(String),
    /// Internal CPAL backend error.
    BackendError(String),
}

impl std::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeviceNotFound(name) => write!(f, "Audio device not found: {name}"),
            Self::NoDefaultDevice(msg) => write!(f, "No default audio device: {msg}"),
            Self::UnsupportedConfig(msg) => write!(f, "Unsupported audio configuration: {msg}"),
            Self::InvalidBufferSize(size) => {
                write!(
                    f,
                    "Invalid buffer size {size} (must be 64, 128, 256, 512, or 1024)"
                )
            }
            Self::InvalidSampleRate(sr) => {
                write!(
                    f,
                    "Invalid sample rate {sr} Hz (must be 44100, 48000, 88200, 96000, or 192000)"
                )
            }
            Self::StreamBuildError(msg) => write!(f, "Failed to build audio stream: {msg}"),
            Self::StreamPlayError(msg) => write!(f, "Failed to play audio stream: {msg}"),
            Self::BackendError(msg) => write!(f, "Audio backend error: {msg}"),
        }
    }
}

impl std::error::Error for DeviceError {}

/// Calculate nominal hardware buffer latency in milliseconds.
#[inline]
pub fn calculate_latency_ms(buffer_size: u32, sample_rate: u32) -> f32 {
    let sr = if sample_rate > 0 {
        sample_rate as f32
    } else {
        DEFAULT_SAMPLE_RATE as f32
    };
    (buffer_size as f32 / sr) * 1000.0
}

/// Format buffer size and latency badge string (e.g. `64 smp - 1.3 ms`).
#[inline]
pub fn latency_badge(buffer_size: u32, sample_rate: u32) -> String {
    let ms = calculate_latency_ms(buffer_size, sample_rate);
    format!("{buffer_size} smp - {ms:.1} ms")
}

/// Validate that a requested buffer size is supported.
pub fn validate_buffer_size(size: u32) -> Result<u32, DeviceError> {
    if SUPPORTED_BUFFER_SIZES.contains(&size) {
        Ok(size)
    } else {
        Err(DeviceError::InvalidBufferSize(size))
    }
}

/// Validate that a requested sample rate is supported.
pub fn validate_sample_rate(rate: u32) -> Result<u32, DeviceError> {
    if SUPPORTED_SAMPLE_RATES.contains(&rate) {
        Ok(rate)
    } else {
        Err(DeviceError::InvalidSampleRate(rate))
    }
}

/// Metadata description of a hardware audio device discovered via CPAL.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AudioDeviceInfo {
    /// Friendly device name reported by driver (e.g. "Vention USB Audio", "Speakers (Realtek)").
    pub name: String,
    /// Whether this is the system default device.
    pub is_default: bool,
    /// True for input devices (mic, line-in), false for output devices (speakers, headphones).
    pub is_input: bool,
    /// Maximum number of channels supported.
    pub max_channels: u16,
    /// Default hardware sample rate in Hz.
    pub default_sample_rate: u32,
    /// List of verified supported sample rates in Hz.
    pub supported_sample_rates: Vec<u32>,
    /// Minimum buffer frame size supported.
    pub min_buffer_size: Option<u32>,
    /// Maximum buffer frame size supported.
    pub max_buffer_size: Option<u32>,
}

/// Audio stream configuration parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AudioStreamConfig {
    /// Sample rate in Hz (e.g. 48000, 44100).
    pub sample_rate: u32,
    /// Buffer frame size (e.g. 64, 128).
    pub buffer_size: u32,
    /// Channel count (default 2 for stereo duplex).
    pub channels: u16,
    /// Request exclusive hardware access (WASAPI Exclusive on Windows).
    pub exclusive: bool,
}

impl AudioStreamConfig {
    /// Default configuration: 48 kHz, 64 buffer frames (~1.3 ms), stereo, exclusive.
    pub const DEFAULT: Self = Self {
        sample_rate: DEFAULT_SAMPLE_RATE,
        buffer_size: DEFAULT_BUFFER_SIZE,
        channels: 2,
        exclusive: true,
    };

    /// Create custom audio configuration with sample rate and buffer frame size.
    pub fn new(sample_rate: u32, buffer_size: u32) -> Self {
        Self {
            sample_rate,
            buffer_size,
            channels: 2,
            exclusive: true,
        }
    }

    /// Calculate buffer latency in milliseconds.
    #[inline]
    pub fn latency_ms(&self) -> f32 {
        calculate_latency_ms(self.buffer_size, self.sample_rate)
    }

    /// Format latency badge string (e.g. `64 smp - 1.3 ms`).
    #[inline]
    pub fn latency_badge(&self) -> String {
        latency_badge(self.buffer_size, self.sample_rate)
    }

    /// Validate configuration against supported ranges.
    pub fn validate(&self) -> Result<(), DeviceError> {
        validate_sample_rate(self.sample_rate)?;
        validate_buffer_size(self.buffer_size)?;
        if self.channels == 0 || self.channels > 8 {
            return Err(DeviceError::UnsupportedConfig(format!(
                "Invalid channel count: {}",
                self.channels
            )));
        }
        Ok(())
    }
}

impl Default for AudioStreamConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Query supported sample rates from a CPAL device.
fn query_supported_sample_rates(
    device: &cpal::Device,
    is_input: bool,
    default_sr: u32,
) -> Vec<u32> {
    let mut rates = Vec::new();

    if is_input {
        if let Ok(configs) = device.supported_input_configs() {
            for range in configs {
                let min = range.min_sample_rate().0;
                let max = range.max_sample_rate().0;
                for &sr in SUPPORTED_SAMPLE_RATES {
                    if sr >= min && sr <= max && !rates.contains(&sr) {
                        rates.push(sr);
                    }
                }
            }
        }
    } else if let Ok(configs) = device.supported_output_configs() {
        for range in configs {
            let min = range.min_sample_rate().0;
            let max = range.max_sample_rate().0;
            for &sr in SUPPORTED_SAMPLE_RATES {
                if sr >= min && sr <= max && !rates.contains(&sr) {
                    rates.push(sr);
                }
            }
        }
    }

    if rates.is_empty() {
        if SUPPORTED_SAMPLE_RATES.contains(&default_sr) {
            rates.push(default_sr);
        } else {
            rates.push(DEFAULT_SAMPLE_RATE);
        }
    }

    rates.sort_unstable();
    rates
}

/// Enumerate all available audio input devices from a CPAL host.
pub fn query_input_devices(host: &cpal::Host) -> Vec<AudioDeviceInfo> {
    let default_name = host.default_input_device().and_then(|d| d.name().ok());
    let mut infos = Vec::new();

    if let Ok(devices) = host.input_devices() {
        for device in devices {
            if let Ok(name) = device.name() {
                let is_default = default_name.as_deref() == Some(&name);
                let default_cfg = device.default_input_config().ok();
                let max_channels = default_cfg.as_ref().map(|c| c.channels()).unwrap_or(2);
                let default_sr = default_cfg
                    .as_ref()
                    .map(|c| c.sample_rate().0)
                    .unwrap_or(DEFAULT_SAMPLE_RATE);
                let supported_srs = query_supported_sample_rates(&device, true, default_sr);

                infos.push(AudioDeviceInfo {
                    name,
                    is_default,
                    is_input: true,
                    max_channels,
                    default_sample_rate: default_sr,
                    supported_sample_rates: supported_srs,
                    min_buffer_size: Some(64),
                    max_buffer_size: Some(1024),
                });
            }
        }
    }

    infos
}

/// Enumerate all available audio output devices from a CPAL host.
pub fn query_output_devices(host: &cpal::Host) -> Vec<AudioDeviceInfo> {
    let default_name = host.default_output_device().and_then(|d| d.name().ok());
    let mut infos = Vec::new();

    if let Ok(devices) = host.output_devices() {
        for device in devices {
            if let Ok(name) = device.name() {
                let is_default = default_name.as_deref() == Some(&name);
                let default_cfg = device.default_output_config().ok();
                let max_channels = default_cfg.as_ref().map(|c| c.channels()).unwrap_or(2);
                let default_sr = default_cfg
                    .as_ref()
                    .map(|c| c.sample_rate().0)
                    .unwrap_or(DEFAULT_SAMPLE_RATE);
                let supported_srs = query_supported_sample_rates(&device, false, default_sr);

                infos.push(AudioDeviceInfo {
                    name,
                    is_default,
                    is_input: false,
                    max_channels,
                    default_sample_rate: default_sr,
                    supported_sample_rates: supported_srs,
                    min_buffer_size: Some(64),
                    max_buffer_size: Some(1024),
                });
            }
        }
    }

    infos
}

/// Find a specific CPAL audio device by name.
pub fn find_device(
    host: &cpal::Host,
    name: &str,
    is_input: bool,
) -> Result<cpal::Device, DeviceError> {
    let devices_res = if is_input {
        host.input_devices()
    } else {
        host.output_devices()
    };

    let devices = devices_res.map_err(|e| DeviceError::BackendError(e.to_string()))?;
    for dev in devices {
        if let Ok(dev_name) = dev.name() {
            if dev_name == name {
                return Ok(dev);
            }
        }
    }

    Err(DeviceError::DeviceNotFound(name.to_string()))
}

/// Get the system default CPAL audio device.
pub fn default_device(host: &cpal::Host, is_input: bool) -> Result<cpal::Device, DeviceError> {
    let dev_opt = if is_input {
        host.default_input_device()
    } else {
        host.default_output_device()
    };

    dev_opt.ok_or_else(|| {
        DeviceError::NoDefaultDevice(if is_input {
            "No default input audio device found".to_string()
        } else {
            "No default output audio device found".to_string()
        })
    })
}
