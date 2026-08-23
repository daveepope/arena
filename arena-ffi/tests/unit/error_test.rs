use arena_ffi::error::{clear_error, write_error};
use arena_ffi::arena_free_string;
use std::ffi::CStr;
use std::os::raw::c_char;

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
