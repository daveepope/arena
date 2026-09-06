use arena_ffi::error::{clear_error, clear_out_string, write_error, write_out_string};
use arena_ffi::arena_free_string;
use std::ffi::CStr;
use std::os::raw::c_char;

fn take(out: *mut c_char) -> String {
    assert!(!out.is_null());
    let value = unsafe { CStr::from_ptr(out) }.to_string_lossy().into_owned();
    arena_free_string(out);
    value
}

#[test]
fn write_error_null_out_does_not_panic() {
    unsafe { write_error(std::ptr::null_mut(), "boom") };
}

#[test]
fn write_error_writes_message_to_out_pointer() {
    let mut err: *mut c_char = std::ptr::null_mut();
    unsafe { write_error(&mut err as *mut _, "boom") };

    assert!(!err.is_null());
    let msg = unsafe { CStr::from_ptr(err) }.to_string_lossy().into_owned();
    assert_eq!(msg, "boom");
    arena_free_string(err);
}

#[test]
fn write_error_message_with_interior_nul_uses_fallback_message() {
    let mut err: *mut c_char = std::ptr::null_mut();
    unsafe { write_error(&mut err as *mut _, "bad\0message") };

    assert!(!err.is_null());
    let msg = unsafe { CStr::from_ptr(err) }.to_string_lossy().into_owned();
    assert_eq!(msg, "arena-ffi: error message contained NUL byte");
    arena_free_string(err);
}

#[test]
fn clear_error_null_out_does_not_panic() {
    unsafe { clear_error(std::ptr::null_mut()) };
}

#[test]
fn clear_error_sets_pointer_to_null() {
    let mut err: *mut c_char = std::ptr::null_mut();
    unsafe { write_error(&mut err as *mut _, "boom") };
    assert!(!err.is_null());

    unsafe { clear_error(&mut err as *mut _) };

    assert!(err.is_null());
}

#[test]
fn write_out_string_writes_value_to_out_pointer() {
    let mut out: *mut c_char = std::ptr::null_mut();

    unsafe { write_out_string(&mut out as *mut _, "{\"id\":\"orders\"}") };

    assert_eq!(take(out), "{\"id\":\"orders\"}");
}

#[test]
fn write_out_string_value_with_interior_nul_uses_fallback_value() {
    let mut out: *mut c_char = std::ptr::null_mut();

    unsafe { write_out_string(&mut out as *mut _, "before\0after") };

    assert_eq!(take(out), "arena-ffi: value contained NUL byte");
}

#[test]
fn write_out_string_null_out_does_not_panic() {
    unsafe { write_out_string(std::ptr::null_mut(), "ignored") };
}

#[test]
fn clear_out_string_sets_pointer_to_null() {
    let mut out: *mut c_char = std::ptr::null_mut();
    unsafe { write_out_string(&mut out as *mut _, "value") };
    let written = out;

    unsafe { clear_out_string(&mut out as *mut _) };

    assert!(out.is_null());
    let _ = take(written);
}

#[test]
fn clear_out_string_null_out_does_not_panic() {
    unsafe { clear_out_string(std::ptr::null_mut()) };
}
