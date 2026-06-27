//! Minimal direct librtlsdr FFI bindings.
//!
//! Safe callers should use `crate::sdr::device` instead of calling these
//! functions directly.

use std::{
    ffi::CStr,
    marker::{PhantomData, PhantomPinned},
    os::raw::{c_char, c_int, c_uint},
    ptr,
    ptr::NonNull,
};

#[repr(C)]
pub struct rtlsdr_dev_t {
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

#[derive(Debug)]
pub(crate) struct DeviceHandle {
    raw: NonNull<rtlsdr_dev_t>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenDeviceError {
    OpenFailed { code: i32 },
    NullHandle,
}

unsafe extern "C" {
    pub fn rtlsdr_get_device_count() -> c_uint;
    pub fn rtlsdr_get_device_name(index: c_uint) -> *const c_char;
    pub fn rtlsdr_get_device_usb_strings(
        index: c_uint,
        manufacturer: *mut c_char,
        product: *mut c_char,
        serial: *mut c_char,
    ) -> c_int;
    pub fn rtlsdr_open(dev: *mut *mut rtlsdr_dev_t, index: c_uint) -> c_int;
    pub fn rtlsdr_close(dev: *mut rtlsdr_dev_t) -> c_int;
    pub fn rtlsdr_set_center_freq(dev: *mut rtlsdr_dev_t, freq: c_uint) -> c_int;
    pub fn rtlsdr_get_center_freq(dev: *mut rtlsdr_dev_t) -> c_uint;
    pub fn rtlsdr_set_sample_rate(dev: *mut rtlsdr_dev_t, rate: c_uint) -> c_int;
    pub fn rtlsdr_get_sample_rate(dev: *mut rtlsdr_dev_t) -> c_uint;
    pub fn rtlsdr_set_tuner_gain_mode(dev: *mut rtlsdr_dev_t, mode: c_int) -> c_int;
    pub fn rtlsdr_set_tuner_gain(dev: *mut rtlsdr_dev_t, gain: c_int) -> c_int;
    pub fn rtlsdr_get_tuner_gain(dev: *mut rtlsdr_dev_t) -> c_int;
}

pub(crate) fn device_name(index: u32) -> String {
    // SAFETY: This reads a process-owned, null-terminated device-name pointer
    // from librtlsdr. Null is treated as an empty device name.
    unsafe { string_from_ptr(rtlsdr_get_device_name(index)) }
}

pub(crate) fn open_device(index: u32) -> Result<DeviceHandle, OpenDeviceError> {
    let mut raw_device = ptr::null_mut();

    // SAFETY: `raw_device` is a valid out-pointer for librtlsdr to initialize
    // during this call. The pointer is checked for null before it leaves this
    // module and is wrapped in `DeviceHandle`.
    let result = unsafe { rtlsdr_open(&mut raw_device, index) };
    if result < 0 {
        return Err(OpenDeviceError::OpenFailed {
            code: result as i32,
        });
    }

    let raw = NonNull::new(raw_device).ok_or(OpenDeviceError::NullHandle)?;

    Ok(DeviceHandle { raw })
}

pub(crate) fn close_device(handle: &mut DeviceHandle) -> i32 {
    // SAFETY: `DeviceHandle` can only be constructed by `open_device`, which
    // guarantees a non-null handle returned by a successful librtlsdr open.
    unsafe { rtlsdr_close(handle.raw.as_ptr()) as i32 }
}

pub(crate) fn set_center_freq(handle: &mut DeviceHandle, frequency_hz: u32) -> i32 {
    // SAFETY: `DeviceHandle` can only be constructed by `open_device`, which
    // guarantees a valid librtlsdr handle for the duration of this call.
    unsafe { rtlsdr_set_center_freq(handle.raw.as_ptr(), frequency_hz) as i32 }
}

pub(crate) fn get_center_freq(handle: &DeviceHandle) -> u32 {
    // SAFETY: `DeviceHandle` can only be constructed by `open_device`, which
    // guarantees a valid librtlsdr handle for the duration of this call.
    unsafe { rtlsdr_get_center_freq(handle.raw.as_ptr()) as u32 }
}

pub(crate) fn set_sample_rate(handle: &mut DeviceHandle, sample_rate_hz: u32) -> i32 {
    // SAFETY: `DeviceHandle` can only be constructed by `open_device`, which
    // guarantees a valid librtlsdr handle for the duration of this call.
    unsafe { rtlsdr_set_sample_rate(handle.raw.as_ptr(), sample_rate_hz) as i32 }
}

pub(crate) fn get_sample_rate(handle: &DeviceHandle) -> u32 {
    // SAFETY: `DeviceHandle` can only be constructed by `open_device`, which
    // guarantees a valid librtlsdr handle for the duration of this call.
    unsafe { rtlsdr_get_sample_rate(handle.raw.as_ptr()) as u32 }
}

pub(crate) fn set_tuner_gain_mode(handle: &mut DeviceHandle, mode: i32) -> i32 {
    // SAFETY: `DeviceHandle` can only be constructed by `open_device`, which
    // guarantees a valid librtlsdr handle for the duration of this call.
    unsafe { rtlsdr_set_tuner_gain_mode(handle.raw.as_ptr(), mode as c_int) as i32 }
}

pub(crate) fn set_tuner_gain(handle: &mut DeviceHandle, gain_tenths_db: i32) -> i32 {
    // SAFETY: `DeviceHandle` can only be constructed by `open_device`, which
    // guarantees a valid librtlsdr handle for the duration of this call.
    unsafe { rtlsdr_set_tuner_gain(handle.raw.as_ptr(), gain_tenths_db as c_int) as i32 }
}

pub(crate) fn get_tuner_gain(handle: &DeviceHandle) -> i32 {
    // SAFETY: `DeviceHandle` can only be constructed by `open_device`, which
    // guarantees a valid librtlsdr handle for the duration of this call.
    unsafe { rtlsdr_get_tuner_gain(handle.raw.as_ptr()) as i32 }
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
}
