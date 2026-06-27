//! Minimal direct librtlsdr FFI bindings.
//!
//! Safe callers should use `crate::sdr::device` instead of calling these
//! functions directly.

use std::os::raw::{c_char, c_int, c_uint};

unsafe extern "C" {
    pub fn rtlsdr_get_device_count() -> c_uint;
    pub fn rtlsdr_get_device_name(index: c_uint) -> *const c_char;
    pub fn rtlsdr_get_device_usb_strings(
        index: c_uint,
        manufacturer: *mut c_char,
        product: *mut c_char,
        serial: *mut c_char,
    ) -> c_int;
}
