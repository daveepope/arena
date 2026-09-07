use std::ffi::CString;
use std::os::raw::c_char;

use arena_ffi::{arena_close, arena_open, ArenaStatus};

#[path = "ffi_error_text.rs"]
mod ffi_error_text;
use ffi_error_text::err_text;

fn open_plain_arena(name: &str) -> *mut arena_ffi::OpenArenaHandle {
    let name = CString::new(name).unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let handle = arena_open(
        name.as_ptr(),
        std::ptr::null(),
        &mut err as *mut _,
        std::ptr::null_mut(),
    );
    assert!(!handle.is_null(), "open failed: {}", err_text(err));
    handle
}

#[test]
fn arena_close_open_arena_returns_terminal_state() {
    let handle = open_plain_arena("close-terminal-state");
    let mut err: *mut c_char = std::ptr::null_mut();
    let mut state: *mut c_char = std::ptr::null_mut();

    let status = arena_close(handle, &mut err as *mut _, &mut state as *mut _);

    assert_eq!(status, ArenaStatus::Ok);
    assert!(err.is_null());
    let parsed: serde_json::Value =
        serde_json::from_str(&err_text(state)).expect("state must be json");
    assert_eq!(parsed["id"], "close-terminal-state");
    assert_eq!(parsed["state"], "arena_closed");
}

#[test]
fn arena_close_null_handle_returns_invalid_argument() {
    let mut err: *mut c_char = std::ptr::null_mut();
    let mut state: *mut c_char = std::ptr::null_mut();

    let status = arena_close(std::ptr::null_mut(), &mut err as *mut _, &mut state as *mut _);

    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(state.is_null());
    assert!(err_text(err).contains("must not be null"));
}

#[test]
fn arena_close_null_out_params_still_closes_the_arena() {
    let handle = open_plain_arena("close-null-out-params");

    let status = arena_close(handle, std::ptr::null_mut(), std::ptr::null_mut());

    assert_eq!(status, ArenaStatus::Ok);
}
