use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::sync::Mutex;

use arena_ffi::{
    arena_add_lifecycle_observer, arena_close, arena_open, arena_remove_lifecycle_observer,
};

#[path = "ffi_error_text.rs"]
mod ffi_error_text;
use ffi_error_text::err_text;

static OBSERVER_API_LOCK: Mutex<()> = Mutex::new(());
static OBSERVED: Mutex<Vec<String>> = Mutex::new(Vec::new());

unsafe extern "C" fn collecting_observer(state_json_utf8: *const c_char, _user_data: *mut c_void) {
    let document = unsafe { CStr::from_ptr(state_json_utf8) }
        .to_string_lossy()
        .into_owned();
    OBSERVED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(document);
}

fn drain_observed() -> Vec<String> {
    let mut guard = OBSERVED.lock().unwrap_or_else(|e| e.into_inner());
    let out = guard.clone();
    guard.clear();
    out
}

fn lifecycle_states(documents: &[String]) -> Vec<String> {
    documents
        .iter()
        .map(|document| {
            let parsed: serde_json::Value =
                serde_json::from_str(document).expect("observed state must be json");
            parsed["state"].as_str().unwrap_or_default().to_string()
        })
        .collect()
}

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

fn close_plain_arena(handle: *mut arena_ffi::OpenArenaHandle) {
    arena_close(handle, std::ptr::null_mut(), std::ptr::null_mut());
}

fn open_and_close_plain_arena(name: &str) {
    close_plain_arena(open_plain_arena(name));
}

#[test]
fn arena_add_lifecycle_observer_open_and_close_reports_in_order() {
    let _guard = OBSERVER_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    drain_observed();
    let token = arena_add_lifecycle_observer(Some(collecting_observer), std::ptr::null_mut());
    assert_ne!(token, 0);

    open_and_close_plain_arena("observer-in-order");
    let observed = drain_observed();
    arena_remove_lifecycle_observer(token);

    let states = lifecycle_states(&observed);
    assert!(!states.is_empty(), "no transitions were observed");
    assert_eq!(states.first().map(String::as_str), Some("arena_starting"));
    assert_eq!(states.last().map(String::as_str), Some("arena_closed"));
    let open_at = states.iter().position(|s| s == "arena_open");
    let closing_at = states.iter().position(|s| s == "arena_closing");
    assert!(
        open_at < closing_at,
        "arena_open must be reported before arena_closing: {states:?}"
    );
}

#[test]
fn arena_add_lifecycle_observer_reports_the_arena_identifier() {
    let _guard = OBSERVER_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    drain_observed();
    let token = arena_add_lifecycle_observer(Some(collecting_observer), std::ptr::null_mut());

    open_and_close_plain_arena("observer-identity");
    let observed = drain_observed();
    arena_remove_lifecycle_observer(token);

    let parsed: serde_json::Value =
        serde_json::from_str(observed.first().expect("at least one transition"))
            .expect("observed state must be json");
    assert_eq!(parsed["id"], "observer-identity");
}

#[test]
fn arena_remove_lifecycle_observer_before_open_reports_nothing() {
    let _guard = OBSERVER_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    drain_observed();
    let token = arena_add_lifecycle_observer(Some(collecting_observer), std::ptr::null_mut());
    arena_remove_lifecycle_observer(token);

    open_and_close_plain_arena("observer-removed-before-open");

    assert!(
        drain_observed().is_empty(),
        "a removed observer must not be reported to"
    );
}

#[test]
fn arena_remove_lifecycle_observer_while_open_stops_transitions() {
    let _guard = OBSERVER_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    drain_observed();
    let token = arena_add_lifecycle_observer(Some(collecting_observer), std::ptr::null_mut());
    let handle = open_plain_arena("observer-removed-while-open");
    assert!(
        !drain_observed().is_empty(),
        "the observer should have been reported to while it was registered"
    );

    arena_remove_lifecycle_observer(token);
    close_plain_arena(handle);

    assert!(
        drain_observed().is_empty(),
        "an observer removed while the arena is open must not be reported to again"
    );
}

#[test]
fn arena_add_lifecycle_observer_null_callback_returns_zero_token() {
    let token = arena_add_lifecycle_observer(None, std::ptr::null_mut());

    assert_eq!(token, 0);
}

#[test]
fn arena_remove_lifecycle_observer_zero_token_keeps_registered_observers() {
    let _guard = OBSERVER_API_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    drain_observed();
    let token = arena_add_lifecycle_observer(Some(collecting_observer), std::ptr::null_mut());

    arena_remove_lifecycle_observer(0);
    open_and_close_plain_arena("observer-zero-token");
    let observed = drain_observed();
    arena_remove_lifecycle_observer(token);

    assert!(
        !observed.is_empty(),
        "a zero token must not remove a registered observer"
    );
}
