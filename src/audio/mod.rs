pub mod device;

pub use device::{
    find_capture_device, find_playback_device, query_capture_devices, query_playback_devices,
    DeviceInfo,
};
