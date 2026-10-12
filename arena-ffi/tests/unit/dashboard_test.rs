use std::ffi::CString;
use std::os::raw::c_char;

use arena_ffi::{
    arena_dashboard_ensure_running, arena_register_dashboard_observer,
    arena_remove_lifecycle_observer,
};

#[path = "ffi_error_text.rs"]
mod ffi_error_text;
use ffi_error_text::err_text;

#[test]
fn arena_register_dashboard_observer_valid_url_returns_removable_token() {
    let url = CString::new("http://127.0.0.1:1").unwrap();
    let token = arena_register_dashboard_observer(url.as_ptr());

    assert_ne!(token, 0);
    arena_remove_lifecycle_observer(token);
}

#[test]
fn arena_register_dashboard_observer_null_url_returns_zero_token() {
    let token = arena_register_dashboard_observer(std::ptr::null());

    assert_eq!(token, 0);
}

#[test]
fn arena_dashboard_ensure_running_no_server_and_no_search_dir_reports_error() {
    let mut err: *mut c_char = std::ptr::null_mut();
    let url = arena_dashboard_ensure_running(1, std::ptr::null(), &mut err as *mut _);

    assert!(url.is_null());
    assert!(!err_text(err).is_empty());
}
