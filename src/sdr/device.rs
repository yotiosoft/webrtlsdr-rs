use std::{fmt, os::raw::c_char};

use super::raw;

const USB_STRING_BUFFER_LEN: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdrDeviceInfo {
    pub index: u32,
    pub name: String,
    pub manufacturer: String,
    pub product: String,
    pub serial: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdrError {
    UsbStringsUnavailable {
        index: u32,
        code: i32,
    },
    OpenFailed {
        index: u32,
        code: i32,
    },
    NullDeviceHandle {
        index: u32,
    },
    SettingFailed {
        index: u32,
        setting: &'static str,
        code: i32,
    },
}

impl fmt::Display for SdrError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UsbStringsUnavailable { index, code } => {
                write!(
                    formatter,
                    "failed to read RTL-SDR USB strings for device {index}: librtlsdr returned {code}"
                )
            }
            Self::OpenFailed { index, code } => {
                write!(
                    formatter,
                    "failed to open RTL-SDR device {index}: librtlsdr returned {code}"
                )
            }
            Self::NullDeviceHandle { index } => {
                write!(
                    formatter,
                    "failed to open RTL-SDR device {index}: librtlsdr returned a null handle"
                )
            }
            Self::SettingFailed {
                index,
                setting,
                code,
            } => {
                write!(
                    formatter,
                    "failed to set RTL-SDR {setting} for device {index}: librtlsdr returned {code}"
                )
            }
        }
    }
}

impl std::error::Error for SdrError {}

pub struct OpenedDevice {
    raw: raw::DeviceHandle,
    info: SdrDeviceInfo,
}

// The handle is only accessed through owned methods and is protected by the
// session mutex when stored in server state. We intentionally do not implement
// Sync, because concurrent direct access to a single librtlsdr handle is not
// part of this abstraction.
unsafe impl Send for OpenedDevice {}

impl OpenedDevice {
    pub fn info(&self) -> &SdrDeviceInfo {
        &self.info
    }

    pub fn set_center_frequency_hz(&mut self, frequency_hz: u32) -> Result<(), SdrError> {
        let code = raw::set_center_freq(&mut self.raw, frequency_hz);
        self.check_setting_result("center frequency", code)
    }

    pub fn center_frequency_hz(&self) -> u32 {
        raw::get_center_freq(&self.raw)
    }

    pub fn set_sample_rate_hz(&mut self, sample_rate_hz: u32) -> Result<(), SdrError> {
        let code = raw::set_sample_rate(&mut self.raw, sample_rate_hz);
        self.check_setting_result("sample rate", code)
    }

    pub fn sample_rate_hz(&self) -> u32 {
        raw::get_sample_rate(&self.raw)
    }

    pub fn set_auto_gain(&mut self) -> Result<(), SdrError> {
        let code = raw::set_tuner_gain_mode(&mut self.raw, 0);
        self.check_setting_result("tuner gain mode", code)
    }

    pub fn set_manual_gain_tenths_db(&mut self, gain_tenths_db: i32) -> Result<(), SdrError> {
        let mode_code = raw::set_tuner_gain_mode(&mut self.raw, 1);
        self.check_setting_result("tuner gain mode", mode_code)?;

        let gain_code = raw::set_tuner_gain(&mut self.raw, gain_tenths_db);
        self.check_setting_result("tuner gain", gain_code)
    }

    pub fn tuner_gain_tenths_db(&self) -> i32 {
        raw::get_tuner_gain(&self.raw)
    }

    fn check_setting_result(&self, setting: &'static str, code: i32) -> Result<(), SdrError> {
        if code < 0 {
            return Err(SdrError::SettingFailed {
                index: self.info.index,
                setting,
                code,
            });
        }

        Ok(())
    }
}

impl fmt::Debug for OpenedDevice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenedDevice")
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

impl Drop for OpenedDevice {
    fn drop(&mut self) {
        let result = raw::close_device(&mut self.raw);
        if result < 0 {
            tracing::warn!(
                code = result,
                index = self.info.index,
                "failed to close RTL-SDR device"
            );
        }
    }
}

pub fn list_devices() -> Result<Vec<SdrDeviceInfo>, SdrError> {
    // SAFETY: This librtlsdr function takes no pointers and returns the current
    // device count. It does not require a device handle.
    let device_count = unsafe { raw::rtlsdr_get_device_count() };
    let mut devices = Vec::with_capacity(device_count as usize);

    for index in 0..device_count {
        devices.push(device_info(index)?);
    }

    Ok(devices)
}

pub fn open_device(index: u32) -> Result<OpenedDevice, SdrError> {
    let info = device_info(index)?;
    let raw = raw::open_device(index).map_err(|error| match error {
        raw::OpenDeviceError::OpenFailed { code } => SdrError::OpenFailed { index, code },
        raw::OpenDeviceError::NullHandle => SdrError::NullDeviceHandle { index },
    })?;

    Ok(OpenedDevice { raw, info })
}

fn device_info(index: u32) -> Result<SdrDeviceInfo, SdrError> {
    let name = raw::device_name(index);

    let mut manufacturer = [0 as c_char; USB_STRING_BUFFER_LEN];
    let mut product = [0 as c_char; USB_STRING_BUFFER_LEN];
    let mut serial = [0 as c_char; USB_STRING_BUFFER_LEN];

    // SAFETY: Each buffer is valid for writes of 256 bytes and is passed only
    // for the duration of the call, as required by librtlsdr.
    let result = unsafe {
        raw::rtlsdr_get_device_usb_strings(
            index,
            manufacturer.as_mut_ptr(),
            product.as_mut_ptr(),
            serial.as_mut_ptr(),
        )
    };
    if result < 0 {
        return Err(SdrError::UsbStringsUnavailable {
            index,
            code: result as i32,
        });
    }

    Ok(SdrDeviceInfo {
        index,
        name,
        manufacturer: string_from_buffer(&manufacturer),
        product: string_from_buffer(&product),
        serial: string_from_buffer(&serial),
    })
}

fn string_from_buffer(buffer: &[c_char]) -> String {
    let nul_position = buffer
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(buffer.len());
    let bytes = &buffer[..nul_position];
    let bytes = bytes.iter().map(|&byte| byte as u8).collect::<Vec<_>>();

    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_from_buffer_stops_at_first_nul() {
        let buffer = ['R' as c_char, 'T' as c_char, 0, 'X' as c_char];

        assert_eq!(string_from_buffer(&buffer), "RT");
    }

    #[test]
    fn string_from_buffer_handles_invalid_utf8_lossily() {
        let buffer = [0xff_u8 as c_char, 0];

        assert_eq!(string_from_buffer(&buffer), "\u{fffd}");
    }

    #[test]
    #[ignore = "requires librtlsdr runtime access and optionally connected RTL-SDR hardware"]
    fn list_devices_with_librtlsdr() {
        let devices = list_devices().expect("RTL-SDR device enumeration should not fail");

        for device in devices {
            println!("{device:?}");
        }
    }
}
