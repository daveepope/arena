use std::ffi::CString;
use std::os::raw::c_char;

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaStatus {
    Ok = 0,
    InvalidArgument = 1,
    Failed = 2,
    Panic = 3,
    NotFound = 4,
}

unsafe fn write_c_string(out: *mut *mut c_char, value: impl Into<String>, fallback: &str) {
    if out.is_null() {
        return;
    }
    let cstring = match CString::new(value.into()) {
        Ok(v) => v,
        Err(_) => CString::new(fallback).unwrap_or_default(),
    };
    unsafe { *out = cstring.into_raw() };
}

pub unsafe fn write_out_string(out: *mut *mut c_char, value: impl Into<String>) {
    unsafe { write_c_string(out, value, "arena-ffi: value contained NUL byte") };
}

pub unsafe fn clear_out_string(out: *mut *mut c_char) {
    if out.is_null() {
        return;
    }
    unsafe { *out = std::ptr::null_mut() };
}

pub unsafe fn write_error(err_out: *mut *mut c_char, message: impl Into<String>) {
    unsafe {
        write_c_string(
            err_out,
            message,
            "arena-ffi: error message contained NUL byte",
        )
    };
}

pub unsafe fn clear_error(err_out: *mut *mut c_char) {
    unsafe { clear_out_string(err_out) };
}
