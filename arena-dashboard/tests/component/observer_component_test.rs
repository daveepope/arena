use std::sync::{Arc, Mutex};

use arena::lifecycle::{ArenaLifecycleObserver, ArenaLifecycleState, ArenaState};
use arena_dashboard::HttpForwardingObserver;
use axum::extract::State;
use axum::routing::post;
use axum::Router;

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
async fn http_forwarding_observer_on_state_forwards_via_http() {
    let (base_url, bodies) = spawn_capture_server().await;

    let observer = HttpForwardingObserver::new(base_url);
    let state = ArenaState::new(
        "http-forward-arena",
        ArenaLifecycleState::ArenaOpen,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    observer.on_state(&state);

    tokio::task::spawn_blocking(move || drop(observer))
        .await
        .expect("drop observer");

    let documents = bodies.lock().unwrap();
    assert_eq!(documents.len(), 1);
    assert!(documents[0].contains("http-forward-arena"));
}
