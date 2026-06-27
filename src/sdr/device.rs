use std::{ffi::CStr, fmt, os::raw::c_char};

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
    UsbStringsUnavailable { index: u32, code: i32 },
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
        }
    }
}

impl std::error::Error for SdrError {}

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

fn device_info(index: u32) -> Result<SdrDeviceInfo, SdrError> {
    // SAFETY: `index` is in the range returned by librtlsdr. A null pointer is
    // handled by `string_from_ptr`.
    let name = unsafe { string_from_ptr(raw::rtlsdr_get_device_name(index)) };

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

unsafe fn string_from_ptr(value: *const c_char) -> String {
    if value.is_null() {
        return String::new();
    }

    // SAFETY: The caller provides a pointer returned by librtlsdr for a
    // null-terminated device name string.
    unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned()
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
    use std::{ffi::CString, ptr};

    #[test]
    fn string_from_ptr_returns_empty_for_null() {
        // SAFETY: This test intentionally verifies that null input is handled.
        let value = unsafe { string_from_ptr(ptr::null()) };

        assert_eq!(value, "");
    }

    #[test]
    fn string_from_ptr_converts_c_string_lossily() {
        let source = CString::new(b"RTL-SDR".as_slice()).expect("valid C string");

        // SAFETY: `source` is a valid null-terminated C string for this scope.
        let value = unsafe { string_from_ptr(source.as_ptr()) };

        assert_eq!(value, "RTL-SDR");
    }

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
