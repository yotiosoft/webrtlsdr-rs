//! SDR backend boundary. Unsafe librtlsdr FFI is isolated under this module.
#[allow(dead_code)]
pub mod device;
#[allow(dead_code)]
pub mod raw;

#[allow(unused_imports)]
pub use device::{OpenedDevice, SdrDeviceInfo, SdrError, list_devices, open_device};
