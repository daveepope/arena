use std::ffi::CString;
use std::os::raw::c_char;

use crate::error::{clear_error, write_error};
use crate::panic_payload::panic_message;
use crate::boundary::call_across_boundary;

#[no_mangle]
pub extern "C" fn arena_oauth_loopback_tls_pem_json(err_out: *mut *mut c_char) -> *mut c_char {
    unsafe { clear_error(err_out) };
    let outcome = call_across_boundary(|| {
        loopback_tls_pem_json_document_or_error(err_out)
    });
    match outcome {
        Ok(pointer) => pointer,
        Err(payload) => {
            unsafe { write_error(err_out, panic_message(payload.as_ref())) };
            std::ptr::null_mut()
        }
    }
}

fn loopback_tls_pem_json_document_or_error(err_out: *mut *mut c_char) -> *mut c_char {
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
