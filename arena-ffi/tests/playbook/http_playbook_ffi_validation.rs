use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use arena_ffi::{arena_free_string, arena_http_playbook_open, arena_http_playbook_verify, ArenaStatus};

fn err_text(err: *mut c_char) -> String {
    if err.is_null() {
        return String::new();
    }
    let msg = unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() };
    arena_free_string(err);
    msg
}

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
