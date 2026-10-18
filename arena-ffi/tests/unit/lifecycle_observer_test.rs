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

fn wait_for_cpu_profile_post(
    listener: &std::net::TcpListener,
    budget: std::time::Duration,
) -> bool {
    listener
        .set_nonblocking(true)
        .expect("set listener non-blocking");
    let deadline = std::time::Instant::now() + budget;
    while std::time::Instant::now() < deadline {
        match listener.accept() {
            Ok((mut stream, _)) => {
                use std::io::{Read, Write};
                let mut buffer = [0u8; 2048];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let _ = stream.write_all(b"HTTP/1.1 202 Accepted\r\ncontent-length: 0\r\n\r\n");
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                if request.starts_with("POST /profile/cpu") {
                    return true;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(_) => return false,
        }
    }
    false
}

fn bind_capture_listener() -> (std::net::TcpListener, std::ffi::CString) {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    let url = std::ffi::CString::new(format!("http://127.0.0.1:{port}")).expect("c string");
    (listener, url)
}

#[test]
fn arena_register_dashboard_observer_validurl_forwardscpuprofileeventsuntilremoved() {
    let (forwarded, forwarded_url) = bind_capture_listener();
    let token = arena_ffi::arena_register_dashboard_observer(forwarded_url.as_ptr());
    assert_ne!(token, 0);

    arena_profile::observer::report(
        "ffi-profile-forwarded",
        arena::lifecycle::RunnableState::Started,
    );
    assert!(
        wait_for_cpu_profile_post(&forwarded, std::time::Duration::from_secs(5)),
        "the dashboard observer did not post a cpu profile event"
    );

    arena_ffi::arena_remove_lifecycle_observer(token);

    let (witness, witness_url) = bind_capture_listener();
    let witness_token = arena_ffi::arena_register_dashboard_observer(witness_url.as_ptr());
    arena_profile::observer::report(
        "ffi-profile-after-remove",
        arena::lifecycle::RunnableState::Started,
    );
    assert!(
        wait_for_cpu_profile_post(&witness, std::time::Duration::from_secs(5)),
        "the observer registered after the removal did not post the cpu profile event"
    );
    arena_ffi::arena_remove_lifecycle_observer(witness_token);

    assert!(
        !wait_for_cpu_profile_post(&forwarded, std::time::Duration::from_millis(250)),
        "a cpu profile event was posted after the observer was removed"
    );
}
