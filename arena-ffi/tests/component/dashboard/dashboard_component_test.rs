use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use arena_ffi::{
    arena_close, arena_dashboard_ensure_running, arena_free_string, arena_open,
    arena_register_dashboard_observer, arena_remove_lifecycle_observer,
};
use axum::extract::State;
use axum::routing::post;
use axum::Router;

fn open_plain_arena(name: &str) -> *mut arena_ffi::OpenArenaHandle {
    let name = CString::new(name).unwrap();
    let mut err: *mut c_char = std::ptr::null_mut();
    let handle = arena_open(
        name.as_ptr(),
        std::ptr::null(),
        &mut err as *mut _,
        std::ptr::null_mut(),
    );
    assert!(!handle.is_null());
    handle
}

fn close_plain_arena(handle: *mut arena_ffi::OpenArenaHandle) {
    arena_close(handle, std::ptr::null_mut(), std::ptr::null_mut());
}

#[derive(Clone, Default)]
struct CaptureState {
    bodies: Arc<Mutex<Vec<String>>>,
}

async fn capture_ingest(State(state): State<CaptureState>, body: String) -> &'static str {
    state.bodies.lock().unwrap().push(body);
    "ok"
}

async fn spawn_capture_server() -> (String, Arc<Mutex<Vec<String>>>) {
    let state = CaptureState::default();
    let bodies = Arc::clone(&state.bodies);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let router = Router::new()
        .route("/ingest", post(capture_ingest))
        .with_state(state);
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    (format!("http://{addr}"), bodies)
}

#[tokio::test]
async fn arena_register_dashboard_observer_open_and_close_forwards_states_via_http() {
    let (base_url, bodies) = spawn_capture_server().await;
    let url = CString::new(base_url).unwrap();
    let token = arena_register_dashboard_observer(url.as_ptr());
    assert_ne!(token, 0);

    let handle_addr = tokio::task::spawn_blocking(|| open_plain_arena("dashboard-ffi-forward") as usize)
        .await
        .expect("join blocking open");
    tokio::task::spawn_blocking(move || {
        close_plain_arena(handle_addr as *mut arena_ffi::OpenArenaHandle)
    })
    .await
    .expect("join blocking close");

    tokio::task::spawn_blocking(move || arena_remove_lifecycle_observer(token))
        .await
        .expect("join blocking remove observer");
    tokio::time::sleep(Duration::from_millis(200)).await;

    let documents = bodies.lock().unwrap();
    assert!(!documents.is_empty(), "no states were forwarded via http");
    assert!(documents.iter().any(|d| d.contains("dashboard-ffi-forward")));
}

#[tokio::test]
async fn arena_dashboard_ensure_running_server_already_listening_with_client_connected_succeeds() {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    tokio::spawn(async move {
        let _ = axum::serve(listener, arena_dashboard::app(arena_dashboard::AppState::new())).await;
    });
    let base_url = format!("http://127.0.0.1:{port}");
    let _connection = reqwest::Client::new()
        .get(format!("{base_url}/events"))
        .send()
        .await
        .expect("connect to events");

    let url = tokio::task::spawn_blocking(move || {
        let mut err: *mut c_char = std::ptr::null_mut();
        let url_ptr = arena_dashboard_ensure_running(port, std::ptr::null(), &mut err as *mut _);
        assert!(!url_ptr.is_null(), "ensure_running failed: {}", unsafe {
            if err.is_null() {
                String::new()
            } else {
                CStr::from_ptr(err).to_string_lossy().into_owned()
            }
        });
        let url = unsafe { CStr::from_ptr(url_ptr) }.to_string_lossy().into_owned();
        arena_free_string(url_ptr);
        url
    })
    .await
    .expect("join blocking ensure_running");

    assert_eq!(url, base_url);
}
