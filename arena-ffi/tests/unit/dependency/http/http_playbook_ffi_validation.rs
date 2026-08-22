use std::ffi::CString;
use std::os::raw::c_char;

use arena_ffi::{arena_http_playbook_open, arena_http_playbook_verify, ArenaStatus};

#[path = "../../ffi_error_text.rs"]
mod ffi_error_text;
use ffi_error_text::err_text;

#[test]
fn http_playbook_open_null_arena_returns_null_and_error() {
    let spec = CString::new(r#"{"dependency_identifier":"dep","mappings":[]}"#).unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let pb = arena_http_playbook_open(
        std::ptr::null_mut(),
        spec.as_ptr(),
        &mut err as *mut _,
    );
    assert!(pb.is_null());
    assert!(err_text(err).contains("arena handle must not be null"));
}

#[test]
fn http_playbook_verify_null_handle_returns_invalid_argument() {
    let spec = CString::new(r#"{"method":"GET","url_path":"/x","expected_count":1}"#).unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let status = arena_http_playbook_verify(std::ptr::null_mut(), spec.as_ptr(), &mut err as *mut _);
    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(err_text(err).contains("handle must not be null"));
}

#[test]
fn http_playbook_open_malformed_spec_json_returns_null_and_parse_error() {
    let spec = CString::new("{not valid json").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let arena_handle = 0x1 as *mut arena_ffi::OpenArenaHandle;
    let pb = arena_http_playbook_open(arena_handle, spec.as_ptr(), &mut err as *mut _);
    assert!(pb.is_null());
    assert!(err_text(err).contains("spec parse failed"));
}

#[test]
fn http_playbook_verify_malformed_spec_json_returns_invalid_argument() {
    let spec = CString::new("{not valid json").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let handle = 0x1 as *mut arena_ffi::ArenaActivePlaybookHandle;
    let status = arena_http_playbook_verify(handle, spec.as_ptr(), &mut err as *mut _);
    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(err_text(err).contains("parse failed"));
}

#[test]
fn http_playbook_verify_both_count_fields_returns_invalid_argument() {
    let spec = CString::new(
        r#"{"method":"GET","url_path":"/x","expected_count":1,"minimum_count":1}"#,
    )
    .unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let status = arena_http_playbook_verify(
        std::ptr::null_mut(),
        spec.as_ptr(),
        &mut err as *mut _,
    );
    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(!err_text(err).is_empty());
}
