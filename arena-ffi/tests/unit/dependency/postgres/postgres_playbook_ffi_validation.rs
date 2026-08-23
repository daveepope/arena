use std::ffi::CString;
use std::os::raw::c_char;

use arena_ffi::{arena_postgres_playbook_verify, ArenaActivePlaybookHandle, ArenaStatus};

#[path = "../../ffi_error_text.rs"]
mod ffi_error_text;
use ffi_error_text::err_text;

#[test]
fn postgres_playbook_verify_null_handle_returns_invalid_argument() {
    let spec = CString::new(r#"{"query":"select 1","expected_value":1}"#).unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let status =
        arena_postgres_playbook_verify(std::ptr::null_mut(), spec.as_ptr(), &mut err as *mut _);
    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(err_text(err).contains("handle must not be null"));
}

#[test]
fn postgres_playbook_verify_malformed_spec_json_returns_invalid_argument() {
    let spec = CString::new("{not valid json").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let handle = 0x1 as *mut ArenaActivePlaybookHandle;
    let status = arena_postgres_playbook_verify(handle, spec.as_ptr(), &mut err as *mut _);
    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(err_text(err).contains("parse failed"));
}

#[test]
fn postgres_playbook_verify_null_spec_returns_invalid_argument() {
    let mut err: *mut c_char = std::ptr::null_mut();
    let handle = 0x1 as *mut ArenaActivePlaybookHandle;
    let status = arena_postgres_playbook_verify(handle, std::ptr::null(), &mut err as *mut _);
    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(err_text(err).contains("verify_spec must not be null"));
}

#[test]
fn postgres_playbook_verify_non_utf8_spec_returns_invalid_argument() {
    let invalid_utf8: &[u8] = &[0x66, 0x6f, 0xff, 0x00];
    let spec_ptr = invalid_utf8.as_ptr() as *const c_char;
    let mut err: *mut c_char = std::ptr::null_mut();
    let handle = 0x1 as *mut ArenaActivePlaybookHandle;
    let status = arena_postgres_playbook_verify(handle, spec_ptr, &mut err as *mut _);
    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(err_text(err).contains("verify_spec is not valid UTF-8"));
}
