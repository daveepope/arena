use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use arena::lifecycle::{ArenaLifecycleObserver, ArenaLifecycleState, ArenaState, RunnableState};
use arena::lifecycle::bus::Observer;
use arena::playbook::ScopedPlaybookEvent;
use arena_dashboard::{HttpForwardingObserver, ScopedPlaybookForwarder};
use arena_profile::{CpuProfileEvent, CpuProfileObserver};
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
        .route(
            "/arena/{arena}/playbook/{playbook}",
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

async fn start_stalling_listener() -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    let accepting = tokio::spawn(async move {
        let mut accepted = Vec::new();
        while let Ok((stream, _)) = listener.accept().await {
            accepted.push(stream);
        }
    });
    (format!("http://127.0.0.1:{port}"), accepting)
}

#[tokio::test(flavor = "multi_thread")]
async fn drop_stalling_dashboard_stops_forwarding_at_the_drain_budget() {
    let (url, accepting) = start_stalling_listener().await;

    let observer = HttpForwardingObserver::new(url);
    for index in 0..10 {
        observer.on_state(&arena_state(&format!("stalled-arena-{index}")));
    }

    let started = Instant::now();
    tokio::task::spawn_blocking(move || drop(observer))
        .await
        .expect("join drop task");
    let elapsed = started.elapsed();
    accepting.abort();

    assert!(
        elapsed < Duration::from_secs(10),
        "teardown took {elapsed:?}, ten stalled posts without a budget would take twenty seconds"
    );
}

async fn start_profile_capture_server(bodies: CapturedBodies) -> String {
    let router = Router::new()
        .route(
            "/profile/cpu",
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

#[tokio::test(flavor = "multi_thread")]
async fn on_cpu_profile_statetransition_postsitwithoutsamples() {
    let bodies: CapturedBodies = Arc::new(Mutex::new(Vec::new()));
    let url = start_profile_capture_server(Arc::clone(&bodies)).await;

    let observer = HttpForwardingObserver::new(url);
    observer.on_cpu_profile(&CpuProfileEvent::new("svc", RunnableState::Started));
    drop(observer);

    let captured = bodies.lock().expect("capture lock").clone();
    assert_eq!(captured.len(), 1);
    let payload: serde_json::Value = serde_json::from_str(&captured[0]).expect("json payload");
    assert_eq!(payload["component"], "svc");
    assert_eq!(payload["state"], "started");
    assert!(payload.get("folded").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn on_cpu_profile_completedprofile_poststhefoldedstacks() {
    let bodies: CapturedBodies = Arc::new(Mutex::new(Vec::new()));
    let url = start_profile_capture_server(Arc::clone(&bodies)).await;

    let observer = HttpForwardingObserver::new(url);
    observer.on_cpu_profile(
        &CpuProfileEvent::new("svc", RunnableState::Stopped).with_folded("main;compute 42\n"),
    );
    drop(observer);

    let captured = bodies.lock().expect("capture lock").clone();
    assert_eq!(captured.len(), 1);
    let payload: serde_json::Value = serde_json::from_str(&captured[0]).expect("json payload");
    assert_eq!(payload["state"], "stopped");
    assert_eq!(payload["folded"], "main;compute 42\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn on_cpu_profile_everytransition_isforwardedinorder() {
    let bodies: CapturedBodies = Arc::new(Mutex::new(Vec::new()));
    let url = start_profile_capture_server(Arc::clone(&bodies)).await;

    let observer = HttpForwardingObserver::new(url);
    for state in [
        RunnableState::Starting,
        RunnableState::Started,
        RunnableState::Stopping,
    ] {
        observer.on_cpu_profile(&CpuProfileEvent::new("svc", state));
    }
    drop(observer);

    let captured = bodies.lock().expect("capture lock").clone();
    let states: Vec<String> = captured
        .iter()
        .map(|body| {
            serde_json::from_str::<serde_json::Value>(body).expect("json")["state"]
                .as_str()
                .expect("state string")
                .to_string()
        })
        .collect();
    assert_eq!(states, vec!["starting", "started", "stopping"]);
}

fn scoped_playbook_event(playbook_id: &str) -> ScopedPlaybookEvent {
    ScopedPlaybookEvent {
        arena_id: "arena-under-test".to_string(),
        playbook_id: playbook_id.to_string(),
        state: RunnableState::Started,
        faults: Vec::new(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn on_event_scoped_playbook_is_forwarded_with_its_arena_and_state() {
    let bodies: CapturedBodies = Arc::new(Mutex::new(Vec::new()));
    let url = start_capture_server(Arc::clone(&bodies)).await;
    let forwarder = Arc::new(HttpForwardingObserver::new(url));
    let playbooks = ScopedPlaybookForwarder::new(Arc::clone(&forwarder));

    playbooks.on_event(&scoped_playbook_event("reset-orders"));
    drop(playbooks);
    drop(forwarder);

    let captured = bodies.lock().expect("capture lock").clone();
    assert_eq!(captured.len(), 1);
    assert!(captured[0].contains("reset-orders"), "got: {}", captured[0]);
    assert!(captured[0].contains("arena-under-test"), "got: {}", captured[0]);
    assert!(captured[0].contains("started"), "got: {}", captured[0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn on_event_scoped_playbook_forwarder_does_not_forward_arena_states() {
    let bodies: CapturedBodies = Arc::new(Mutex::new(Vec::new()));
    let url = start_capture_server(Arc::clone(&bodies)).await;
    let forwarder = Arc::new(HttpForwardingObserver::new(url));
    let playbooks = ScopedPlaybookForwarder::new(Arc::clone(&forwarder));

    playbooks.on_event(&scoped_playbook_event("reset-orders"));
    playbooks.on_event(&scoped_playbook_event("seed-reference-data"));
    drop(playbooks);
    drop(forwarder);

    let captured = bodies.lock().expect("capture lock").clone();
    assert_eq!(captured.len(), 2);
    assert!(captured.iter().all(|body| body.contains("playbook_id")));
}

#[tokio::test(flavor = "multi_thread")]
async fn on_event_every_channel_shares_one_forwarding_thread() {
    let bodies: CapturedBodies = Arc::new(Mutex::new(Vec::new()));
    let url = start_capture_server(Arc::clone(&bodies)).await;
    let forwarder = Arc::new(HttpForwardingObserver::new(url));
    let playbooks = ScopedPlaybookForwarder::new(Arc::clone(&forwarder));

    forwarder.on_state(&arena_state("shared-arena"));
    playbooks.on_event(&scoped_playbook_event("reset-orders"));
    drop(playbooks);
    drop(forwarder);

    let captured = bodies.lock().expect("capture lock").clone();
    assert_eq!(captured.len(), 2);
    assert!(captured[0].contains("shared-arena"));
    assert!(captured[1].contains("reset-orders"));
}
