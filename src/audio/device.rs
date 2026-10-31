use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

/// Audio device metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub index: usize,
    pub name: String,
    pub is_default: bool,
}

/// Enumerate all input (capture) audio devices for the given host.
pub fn query_capture_devices(host: &cpal::Host) -> Vec<DeviceInfo> {
    let default_name = host.default_input_device().and_then(|d| d.name().ok());
    let mut result = Vec::new();

    if let Ok(devices) = host.input_devices() {
        for (index, dev) in devices.enumerate() {
            if let Ok(name) = dev.name() {
                let is_default = default_name.as_ref() == Some(&name);
                result.push(DeviceInfo {
                    index,
                    name,
                    is_default,
                });
            }
        }
    }

    result
}

/// Enumerate all output (playback) audio devices for the given host.
pub fn query_playback_devices(host: &cpal::Host) -> Vec<DeviceInfo> {
    let default_name = host.default_output_device().and_then(|d| d.name().ok());
    let mut result = Vec::new();

    if let Ok(devices) = host.output_devices() {
        for (index, dev) in devices.enumerate() {
            if let Ok(name) = dev.name() {
                let is_default = default_name.as_ref() == Some(&name);
                result.push(DeviceInfo {
                    index,
                    name,
                    is_default,
                });
            }
        }
    }

    result
}

/// Find a device in a slice of `DeviceInfo` by case-insensitive name match.
/// First looks for an exact case-insensitive match, then substring match.
pub fn find_in_devices(devices: &[DeviceInfo], query: &str) -> Option<DeviceInfo> {
    let query_lower = query.to_lowercase();

    // Try exact case-insensitive match first
    if let Some(dev) = devices
        .iter()
        .find(|d| d.name.to_lowercase() == query_lower)
    {
        return Some(dev.clone());
    }

    // Fall back to case-insensitive substring match
    devices
        .iter()
        .find(|d| d.name.to_lowercase().contains(&query_lower))
        .cloned()
}

/// Find a capture device on the host by name (case-insensitive substring/exact).
pub fn find_capture_device(host: &cpal::Host, query: &str) -> Option<DeviceInfo> {
    let devices = query_capture_devices(host);
    find_in_devices(&devices, query)
}

/// Find a playback device on the host by name (case-insensitive substring/exact).
pub fn find_playback_device(host: &cpal::Host, query: &str) -> Option<DeviceInfo> {
    let devices = query_playback_devices(host);
    find_in_devices(&devices, query)
}
