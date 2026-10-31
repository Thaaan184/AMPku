use cpal::traits::{DeviceTrait, HostTrait};
use cpal::Host;

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub index: usize,
    pub name: String,
    pub is_default: bool,
}

/// Enumerate all audio input (capture) devices.
pub fn get_capture_devices(host: &Host) -> Vec<DeviceInfo> {
    let default_name = host.default_input_device().and_then(|d| d.name().ok());
    let mut list = Vec::new();

    if let Ok(devices) = host.input_devices() {
        for (i, dev) in devices.enumerate() {
            if let Ok(name) = dev.name() {
                let is_default = default_name.as_deref() == Some(&name);
                list.push(DeviceInfo {
                    index: i,
                    name,
                    is_default,
                });
            }
        }
    }
    list
}

/// Enumerate all audio output (playback) devices.
pub fn get_playback_devices(host: &Host) -> Vec<DeviceInfo> {
    let default_name = host.default_output_device().and_then(|d| d.name().ok());
    let mut list = Vec::new();

    if let Ok(devices) = host.output_devices() {
        for (i, dev) in devices.enumerate() {
            if let Ok(name) = dev.name() {
                let is_default = default_name.as_deref() == Some(&name);
                list.push(DeviceInfo {
                    index: i,
                    name,
                    is_default,
                });
            }
        }
    }
    list
}

/// Find device matching pattern (case-insensitive substring match).
pub fn find_device_by_name(devices: &[DeviceInfo], pattern: &str) -> Option<DeviceInfo> {
    let pat = pattern.to_lowercase();
    devices
        .iter()
        .find(|d| d.name.to_lowercase().contains(&pat))
        .cloned()
}

/// Resolve CPAL Device for a given input device name.
pub fn get_input_device_by_name(host: &Host, name: &str) -> Option<cpal::Device> {
    host.input_devices()
        .ok()
        .and_then(|mut devs| devs.find(|d| d.name().map(|n| n == name).unwrap_or(false)))
}

/// Resolve CPAL Device for a given output device name.
pub fn get_output_device_by_name(host: &Host, name: &str) -> Option<cpal::Device> {
    host.output_devices()
        .ok()
        .and_then(|mut devs| devs.find(|d| d.name().map(|n| n == name).unwrap_or(false)))
}
