use std::ffi::CString;
use std::os::raw::c_char;

use arena_ffi::{arena_close, arena_open, arena_state_json, ArenaStatus};

#[path = "ffi_error_text.rs"]
mod ffi_error_text;
use ffi_error_text::err_text;

fn open_plain_arena(name: &str) -> (*mut arena_ffi::OpenArenaHandle, String) {
    let name = CString::new(name).unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let mut state: *mut c_char = std::ptr::null_mut();
    let handle = arena_open(
        name.as_ptr(),
        std::ptr::null(),
        &mut err as *mut _,
        &mut state as *mut _,
    );
    assert!(!handle.is_null(), "open failed: {}", err_text(err));
    (handle, err_text(state))
}

#[test]
fn arena_open_success_writes_state_document() {
    let (handle, state) = open_plain_arena("state-open-success");
    arena_close(handle, std::ptr::null_mut(), std::ptr::null_mut());

    let parsed: serde_json::Value = serde_json::from_str(&state).expect("state must be json");
    assert_eq!(parsed["id"], "state-open-success");
    assert_eq!(parsed["state"], "arena_open");
    assert!(parsed["faults"].as_array().expect("faults array").is_empty());
}

#[test]
fn arena_open_build_failure_writes_no_state_and_returns_null_handle() {
    let name = CString::new("state-open-build-failure").unwrap();
    let config = CString::new(
        r#"{"dependencies":[{"type":"kafka","identifier":"state-open-kafka","flavor":"nope"}]}"#,
    )
    .unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let mut state: *mut c_char = std::ptr::null_mut();

    let handle = arena_open(
        name.as_ptr(),
        config.as_ptr(),
        &mut err as *mut _,
        &mut state as *mut _,
    );

    assert!(handle.is_null());
    assert!(state.is_null(), "a build failure has no arena to report");
    assert!(!err_text(err).is_empty());
}

#[test]
fn arena_state_json_live_arena_returns_state_document() {
    let (handle, _) = open_plain_arena("state-live");
    let mut err: *mut c_char = std::ptr::null_mut();
    let mut state: *mut c_char = std::ptr::null_mut();

    let status = arena_state_json(handle, &mut err as *mut _, &mut state as *mut _);

    assert_eq!(status, ArenaStatus::Ok);
    let document = err_text(state);
    arena_close(handle, std::ptr::null_mut(), std::ptr::null_mut());
    let parsed: serde_json::Value = serde_json::from_str(&document).expect("state must be json");
    assert_eq!(parsed["id"], "state-live");
    assert_eq!(parsed["state"], "arena_open");
}

#[test]
fn arena_state_json_null_handle_returns_invalid_argument() {
    let mut err: *mut c_char = std::ptr::null_mut();
    let mut state: *mut c_char = std::ptr::null_mut();

    let status = arena_state_json(std::ptr::null_mut(), &mut err as *mut _, &mut state as *mut _);

    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(state.is_null());
    assert!(err_text(err).contains("must not be null"));
}



const MATCH_JSON_COMPONENT_THAT_CANNOT_START: &str = r#"{"components":[{"type":"exec","identifier":"state-missing-binary","executable_path":"/nonexistent/arena-state-probe-binary"}]}"#;

#[test]
fn arena_open_faulted_writes_state_and_null_handle() {
    let name = CString::new("state-open-faulted").unwrap();
    let config = CString::new(MATCH_JSON_COMPONENT_THAT_CANNOT_START).unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let mut state: *mut c_char = std::ptr::null_mut();

    let handle = arena_open(
        name.as_ptr(),
        config.as_ptr(),
        &mut err as *mut _,
        &mut state as *mut _,
    );

    assert!(handle.is_null());
    assert!(!err_text(err).is_empty());
    let parsed: serde_json::Value =
        serde_json::from_str(&err_text(state)).expect("state must be json");
    assert_eq!(parsed["id"], "state-open-faulted");
    assert_eq!(parsed["state"], "arena_faulted");
    assert!(
        !parsed["faults"].as_array().expect("faults array").is_empty(),
        "a faulted open must report at least one fault"
    );
    let component_id = parsed["components"][0]["id"]
        .as_str()
        .expect("component id");
    assert!(
        component_id.contains("state-missing-binary"),
        "component id should carry the configured identifier: {component_id}"
    );
}
