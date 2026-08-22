use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use arena_ffi::{
    arena_active_playbook_drop, arena_close, arena_free_string, arena_match_playbook_run, arena_open, ArenaStatus,
};

fn err_text(err: *mut c_char) -> String {
    if err.is_null() {
        return String::new();
    }
    let msg = unsafe { CStr::from_ptr(err) }.to_string_lossy().into_owned();
    arena_free_string(err);
    msg
}

#[test]
fn arena_match_playbook_run_null_arena_handle_returns_null_and_writes_error() {
    let identifier = CString::new("pb").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();

    let handle = arena_match_playbook_run(std::ptr::null_mut(), identifier.as_ptr(), &mut err as *mut _);

    assert!(handle.is_null());
    assert!(err_text(err).contains("arena handle must not be null"));
}

#[test]
fn arena_match_playbook_run_null_identifier_returns_null_and_writes_error() {
    let name = CString::new("test").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let arena_handle = arena_open(name.as_ptr(), std::ptr::null(), &mut err as *mut _);
    assert!(!arena_handle.is_null());

    let handle = arena_match_playbook_run(arena_handle, std::ptr::null(), &mut err as *mut _);

    assert!(handle.is_null());
    assert!(err_text(err).contains("identifier must not be null"));
    arena_close(arena_handle);
}

#[test]
fn arena_match_playbook_run_unregistered_playbook_returns_null_and_writes_error() {
    let name = CString::new("test").unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let arena_handle = arena_open(name.as_ptr(), std::ptr::null(), &mut err as *mut _);
    assert!(!arena_handle.is_null());

    let identifier = CString::new("does-not-exist").unwrap();
    let handle = arena_match_playbook_run(arena_handle, identifier.as_ptr(), &mut err as *mut _);

    assert!(handle.is_null());
    assert!(err_text(err).contains("is not registered on any match"));
    arena_close(arena_handle);
}

#[test]
fn arena_active_playbook_drop_null_handle_returns_ok() {
    let mut err: *mut c_char = std::ptr::null_mut();

    let status = arena_active_playbook_drop(std::ptr::null_mut(), &mut err as *mut _);

    assert_eq!(status, ArenaStatus::Ok);
    assert!(err.is_null());
}
