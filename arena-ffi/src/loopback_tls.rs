use std::ffi::CString;
use std::os::raw::c_char;

use crate::error::{clear_error, write_error};

#[no_mangle]
pub extern "C" fn arena_oauth_loopback_tls_pem_json(err_out: *mut *mut c_char) -> *mut c_char {
    unsafe { clear_error(err_out) };
    match arena_oauth::loopback_tls_pem_json_document() {
        Ok(document) => match CString::new(document) {
            Ok(c) => c.into_raw(),
            Err(_) => {
                unsafe { write_error(err_out, "loopback tls pem document contained interior NUL") };
                std::ptr::null_mut()
            }
        },
        Err(message) => {
            unsafe { write_error(err_out, message) };
            std::ptr::null_mut()
        }
    }
}
