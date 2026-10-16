use std::sync::{Arc, Mutex};

use arena::lifecycle::{ArenaLifecycleObserver, ArenaLifecycleState, ArenaState};
use arena_dashboard::HttpForwardingObserver;
use axum::extract::State;
use axum::routing::post;
use axum::Router;

type CapturedBodies = Arc<Mutex<Vec<String>>>;

async fn start_capture_server(bodies: CapturedBodies) -> String {
    let router = Router::new()
        .route(
            "/ingest",
            post(|State(captured): State<CapturedBodies>, body: String| async move {
                captured.lock().expect("capture lock").push(body);
            }),
        )
        .with_state(bodies);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    format!("http://127.0.0.1:{port}")
}

fn arena_state(id: &str) -> ArenaState {
    ArenaState::new(
        id,
        ArenaLifecycleState::ArenaOpen,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn on_state_reachable_dashboard_forwards_the_state_document_to_ingest() {
    let bodies: CapturedBodies = Arc::new(Mutex::new(Vec::new()));
    let url = start_capture_server(Arc::clone(&bodies)).await;

    let observer = HttpForwardingObserver::new(url);
    observer.on_state(&arena_state("forwarded-arena"));
    drop(observer);

    let captured = bodies.lock().expect("capture lock").clone();
    assert_eq!(captured.len(), 1);
    assert!(captured[0].contains("forwarded-arena"), "got: {}", captured[0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn on_state_trailing_slash_in_url_still_posts_to_a_single_ingest_path() {
    let bodies: CapturedBodies = Arc::new(Mutex::new(Vec::new()));
    let url = start_capture_server(Arc::clone(&bodies)).await;

    let observer = HttpForwardingObserver::new(format!("{url}/"));
    observer.on_state(&arena_state("trailing-slash-arena"));
    drop(observer);

    assert_eq!(bodies.lock().expect("capture lock").len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn on_state_every_update_is_forwarded_in_order() {
    let bodies: CapturedBodies = Arc::new(Mutex::new(Vec::new()));
    let url = start_capture_server(Arc::clone(&bodies)).await;

    let observer = HttpForwardingObserver::new(url);
    observer.on_state(&arena_state("first-arena"));
    observer.on_state(&arena_state("second-arena"));
    drop(observer);

    let captured = bodies.lock().expect("capture lock").clone();
    assert_eq!(captured.len(), 2);
    assert!(captured[0].contains("first-arena"));
    assert!(captured[1].contains("second-arena"));
}

#[tokio::test]
async fn drop_unreachable_dashboard_completes_without_blocking_teardown() {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);

    let observer = HttpForwardingObserver::new(format!("http://127.0.0.1:{port}"));
    observer.on_state(&arena_state("unreachable-arena"));

    let dropped = tokio::task::spawn_blocking(move || drop(observer));

    tokio::time::timeout(std::time::Duration::from_secs(20), dropped)
        .await
        .expect("drop must not block teardown")
        .expect("join drop task");
}
