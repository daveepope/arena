use std::sync::Arc;
use std::time::Duration;

use arena_dashboard::{app, AppState, ServerShutdown, CALLER_HEADER, CALLER_HEADER_VALUE};
use futures::StreamExt;

async fn spawn_server() -> String {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let state = AppState::new();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app(state)).await;
    });
    format!("http://{addr}")
}

async fn spawn_server_with_shutdown() -> (String, Arc<ServerShutdown>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let state = AppState::new();
    let shutdown = state.shutdown_handle();
    let shutdown_wait = Arc::clone(&shutdown);
    let serve_task = tokio::spawn(async move {
        let _ = axum::serve(listener, app(state))
            .with_graceful_shutdown(async move { shutdown_wait.wait().await })
            .await;
    });
    (format!("http://{addr}"), shutdown, serve_task)
}

#[tokio::test]
async fn ingest_validstate_appearsonevents() {
    let base_url = spawn_server().await;
    let client = reqwest::Client::new();

    let events_response = client
        .get(format!("{base_url}/events"))
        .send()
        .await
        .expect("connect to events");
    let mut stream = events_response.bytes_stream();

    let body = r#"{"id":"test-arena","state":"arena_open","at":"2026-01-01T00:00:00.000Z"}"#;
    let post_response = client
        .post(format!("{base_url}/ingest"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE)
        .body(body)
        .send()
        .await
        .expect("post ingest");
    assert_eq!(post_response.status(), 202);

    let chunk = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("sse event within timeout")
        .expect("sse stream item")
        .expect("sse chunk bytes");
    let text = String::from_utf8_lossy(&chunk);
    assert!(text.contains("test-arena"));
}

#[tokio::test]
async fn events_connect_after_two_arenas_ingested_replays_in_insertion_order() {
    let base_url = spawn_server().await;
    let client = reqwest::Client::new();

    let first = r#"{"id":"arena-first","state":"arena_open","at":"2026-01-01T00:00:00.000Z"}"#;
    let second = r#"{"id":"arena-second","state":"arena_open","at":"2026-01-01T00:00:01.000Z"}"#;
    client.post(format!("{base_url}/ingest"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE).body(first).send().await.expect("post first");
    client.post(format!("{base_url}/ingest"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE).body(second).send().await.expect("post second");

    let events_response = client
        .get(format!("{base_url}/events"))
        .send()
        .await
        .expect("connect to events");
    let mut stream = events_response.bytes_stream();

    let chunk1 = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("sse event within timeout")
        .expect("sse stream item")
        .expect("sse chunk bytes");
    let chunk2 = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("sse event within timeout")
        .expect("sse stream item")
        .expect("sse chunk bytes");

    assert!(String::from_utf8_lossy(&chunk1).contains("arena-first"));
    assert!(String::from_utf8_lossy(&chunk2).contains("arena-second"));
}

#[tokio::test]
async fn events_connect_after_many_transitions_for_one_arena_replays_only_the_latest() {
    let base_url = spawn_server().await;
    let client = reqwest::Client::new();

    let transitions = [
        "arena_starting",
        "dependencies_starting",
        "dependencies_started",
        "components_starting",
        "components_started",
        "arena_open",
    ];
    for (i, state) in transitions.iter().enumerate() {
        let body = format!(
            r#"{{"id":"lifecycle-arena","state":"{state}","at":"2026-01-01T00:00:0{i}.000Z"}}"#
        );
        client.post(format!("{base_url}/ingest"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE).body(body).send().await.expect("post ingest");
    }

    let events_response = client
        .get(format!("{base_url}/events"))
        .send()
        .await
        .expect("connect to events");
    let mut stream = events_response.bytes_stream();

    let chunk = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("sse event within timeout")
        .expect("sse stream item")
        .expect("sse chunk bytes");
    let text = String::from_utf8_lossy(&chunk);
    assert!(text.contains("arena_open"), "{text}");

    let no_further_replay = tokio::time::timeout(Duration::from_millis(300), stream.next()).await;
    assert!(
        no_further_replay.is_err(),
        "connect replay should only send the latest state per arena, not the full history"
    );
}

#[tokio::test]
async fn clear_afteringest_broadcastsclearandemptiesreplay() {
    let base_url = spawn_server().await;
    let client = reqwest::Client::new();

    let body = r#"{"id":"test-arena","state":"arena_open","at":"2026-01-01T00:00:00.000Z"}"#;
    client.post(format!("{base_url}/ingest"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE).body(body).send().await.expect("post ingest");

    let events_response = client
        .get(format!("{base_url}/events"))
        .send()
        .await
        .expect("connect to events");
    let mut stream = events_response.bytes_stream();

    let replay_chunk = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("sse event within timeout")
        .expect("sse stream item")
        .expect("sse chunk bytes");
    assert!(String::from_utf8_lossy(&replay_chunk).contains("test-arena"));

    let clear_response = client.post(format!("{base_url}/clear"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE).send().await.expect("post clear");
    assert_eq!(clear_response.status(), 204);

    let clear_chunk = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("sse event within timeout")
        .expect("sse stream item")
        .expect("sse chunk bytes");
    assert!(String::from_utf8_lossy(&clear_chunk).contains(r#""type":"clear""#));

    let replay_after_clear = client
        .get(format!("{base_url}/events"))
        .send()
        .await
        .expect("connect to events again");
    let mut stream_after_clear = replay_after_clear.bytes_stream();
    let first_after_clear = tokio::time::timeout(Duration::from_secs(5), stream_after_clear.next())
        .await
        .expect("frame within timeout")
        .expect("sse stream item")
        .expect("sse chunk bytes");
    let first_frame = String::from_utf8_lossy(&first_after_clear);
    assert!(
        first_frame.contains("event: heartbeat"),
        "expected only a heartbeat on a cleared server, got: {first_frame}"
    );
    assert!(
        !first_frame.contains("test-arena"),
        "replay buffer should be empty after clear"
    );
}

#[tokio::test]
async fn shutdown_postendpoint_stopsserverfromaccepting() {
    let (base_url, _shutdown, _serve_task) = spawn_server_with_shutdown().await;
    let client = reqwest::Client::new();

    let response = client.post(format!("{base_url}/shutdown"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE).send().await.expect("post shutdown");
    assert_eq!(response.status(), 202);

    tokio::time::sleep(Duration::from_millis(200)).await;

    let result = client
        .get(&base_url)
        .timeout(Duration::from_millis(500))
        .send()
        .await;
    assert!(result.is_err(), "server should have stopped accepting connections after shutdown");
}

#[tokio::test]
async fn ingest_malformedbody_returnsbadrequest() {
    let base_url = spawn_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base_url}/ingest"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE)
        .body("not json")
        .send()
        .await
        .expect("post ingest");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn root_request_servesdashboarddocument() {
    let base_url = spawn_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(base_url)
        .send()
        .await
        .expect("request root");

    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("read body");
    assert!(body.contains("Arena Topology Dashboard"));
    assert!(body.contains("bundle.js"), "got: {body}");
}

#[tokio::test]
async fn events_idleconnection_heartbeatframecarriesnonemptydata() {
    let base_url = spawn_server().await;
    let client = reqwest::Client::new();

    let events_response = client
        .get(format!("{base_url}/events"))
        .send()
        .await
        .expect("connect to events");
    let mut stream = events_response.bytes_stream();

    let mut text = String::new();
    let frame = loop {
        let chunk = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .expect("heartbeat frame within timeout")
            .expect("sse stream item")
            .expect("sse chunk bytes");
        text.push_str(&String::from_utf8_lossy(&chunk));
        if let Some(frame) = text
            .split("\n\n")
            .find(|frame| frame.contains("event: heartbeat"))
        {
            break frame.to_string();
        }
    };

    let payload = frame
        .lines()
        .find_map(|line| line.strip_prefix("data:"))
        .expect("heartbeat frame carries a data field");
    assert!(!payload.trim().is_empty());
}

#[tokio::test]
async fn shutdown_witheventsclientconnected_endsservertask() {
    let (base_url, _shutdown, serve_task) = spawn_server_with_shutdown().await;
    let client = reqwest::Client::new();

    let events_response = client
        .get(format!("{base_url}/events"))
        .send()
        .await
        .expect("connect to events");
    let mut stream = events_response.bytes_stream();
    tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("first frame within timeout")
        .expect("sse stream item")
        .expect("sse chunk bytes");

    client
        .post(format!("{base_url}/shutdown"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE)
        .send()
        .await
        .expect("post shutdown");

    tokio::time::timeout(Duration::from_secs(5), serve_task)
        .await
        .expect("server task ends while an events client is still connected")
        .expect("join server task");
}

#[tokio::test]
async fn history_afterseveralingests_returnsallstatesinorder() {
    let base_url = spawn_server().await;
    let client = reqwest::Client::new();

    for state in ["arena_starting", "arena_open", "arena_closed"] {
        let body = format!(
            r#"{{"id":"history-arena","state":"{state}","at":"2026-01-01T00:00:00.000Z"}}"#
        );
        let response = client
            .post(format!("{base_url}/ingest"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE)
            .body(body)
            .send()
            .await
            .expect("post ingest");
        assert_eq!(response.status(), 202);
    }

    let history: serde_json::Value = client
        .get(format!("{base_url}/history"))
        .send()
        .await
        .expect("request history")
        .json()
        .await
        .expect("history json");

    let arena = &history[0];
    assert_eq!(arena["id"], "history-arena");
    let states = arena["states"].as_array().expect("states array");
    assert_eq!(states.len(), 3);
    assert_eq!(states[0]["state"], "arena_starting");
    assert_eq!(states[1]["state"], "arena_open");
    assert_eq!(states[2]["state"], "arena_closed");
}

#[tokio::test]
async fn history_afterclear_returnsnoarenas() {
    let base_url = spawn_server().await;
    let client = reqwest::Client::new();

    let body = r#"{"id":"history-cleared-arena","state":"arena_open","at":"2026-01-01T00:00:00.000Z"}"#;
    client
        .post(format!("{base_url}/ingest"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE)
        .body(body)
        .send()
        .await
        .expect("post ingest");
    client
        .post(format!("{base_url}/clear"))
        .header(CALLER_HEADER, CALLER_HEADER_VALUE)
        .send()
        .await
        .expect("post clear");

    let history: serde_json::Value = client
        .get(format!("{base_url}/history"))
        .send()
        .await
        .expect("request history")
        .json()
        .await
        .expect("history json");

    assert_eq!(history.as_array().expect("history array").len(), 0);
}
