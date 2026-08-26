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

pub unsafe fn write_error(err_out: *mut *mut c_char, message: impl Into<String>) {
    if err_out.is_null() {
        return;
    }
    let cstring = match CString::new(message.into()) {
        Ok(v) => v,
        Err(_) => CString::new("arena-ffi: error message contained NUL byte").unwrap(),
    };
    unsafe { *err_out = cstring.into_raw() };
}

pub unsafe fn clear_error(err_out: *mut *mut c_char) {
    if err_out.is_null() {
        return;
    }
    unsafe { *err_out = std::ptr::null_mut() };
}
