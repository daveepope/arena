use std::sync::{Arc, Mutex};

use arena_dashboard::{app, live_event_stream, record_history, AppState, HISTORY_LIMIT_PER_ARENA};
use futures::StreamExt;
use tokio::sync::broadcast;

#[tokio::test]
async fn live_event_stream_lagged_resyncs_to_latest_state() {
    let (tx, rx) = broadcast::channel::<String>(4);
    let last_state: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));

    for i in 0..10 {
        let document = format!(
            r#"{{"id":"lag-arena","state":"state-{i}","at":"2026-01-01T00:00:00.000Z"}}"#
        );
        last_state.lock().unwrap().clear();
        last_state.lock().unwrap().push(("lag-arena".to_string(), document.clone()));
        let _ = tx.send(document);
    }

    let mut stream = Box::pin(live_event_stream(rx, Arc::clone(&last_state)));
    let event = stream.next().await.expect("stream yields an event").expect("ok event");
    let data = format!("{event:?}");

    assert!(data.contains("state-9"), "expected resync to latest state, got: {data}");
}

#[tokio::test]
async fn live_event_stream_not_lagged_delivers_each_message() {
    let (tx, rx) = broadcast::channel::<String>(4);
    let last_state: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));

    let _ = tx.send("first".to_string());
    let _ = tx.send("second".to_string());

    let mut stream = Box::pin(live_event_stream(rx, last_state));
    let first = stream.next().await.expect("stream yields an event").expect("ok event");
    let second = stream.next().await.expect("stream yields an event").expect("ok event");

    assert!(format!("{first:?}").contains("first"));
    assert!(format!("{second:?}").contains("second"));
}

#[test]
fn record_history_newarena_startsitsownlist() {
    let mut history: Vec<(String, Vec<String>)> = Vec::new();

    record_history(&mut history, "arena-a", "first".to_string());
    record_history(&mut history, "arena-b", "other".to_string());

    assert_eq!(history.len(), 2);
    assert_eq!(history[0], ("arena-a".to_string(), vec!["first".to_string()]));
    assert_eq!(history[1], ("arena-b".to_string(), vec!["other".to_string()]));
}

#[test]
fn record_history_samearena_appendsinarrivalorder() {
    let mut history: Vec<(String, Vec<String>)> = Vec::new();

    record_history(&mut history, "arena-a", "first".to_string());
    record_history(&mut history, "arena-a", "second".to_string());
    record_history(&mut history, "arena-a", "third".to_string());

    assert_eq!(history[0].1, vec!["first", "second", "third"]);
}

#[test]
fn record_history_pastlimit_dropsoldestandkeepsnewest() {
    let mut history: Vec<(String, Vec<String>)> = Vec::new();
    let overflow = 5;

    for i in 0..HISTORY_LIMIT_PER_ARENA + overflow {
        record_history(&mut history, "arena-a", format!("state-{i}"));
    }

    let bodies = &history[0].1;
    assert_eq!(bodies.len(), HISTORY_LIMIT_PER_ARENA);
    assert_eq!(bodies[0], format!("state-{overflow}"));
    assert_eq!(
        bodies[bodies.len() - 1],
        format!("state-{}", HISTORY_LIMIT_PER_ARENA + overflow - 1)
    );
}

async fn serve_embedded_assets() -> String {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app(AppState::new())).await;
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn app_indexrequest_servesembeddeddocument() {
    let base_url = serve_embedded_assets().await;

    let response = reqwest::get(format!("{base_url}/"))
        .await
        .expect("request index");
    assert!(response.status().is_success());
    let body = response.text().await.expect("index body");

    assert!(body.contains("<div id=\"root\">"), "got: {body}");
    assert!(body.contains("bundle.js"), "got: {body}");
    assert!(
        !body.contains("no web assets embedded"),
        "served the unembedded placeholder instead of the real document: {body}"
    );
}

#[tokio::test]
async fn app_assetrequests_serveembeddedcontentwithtype() {
    let base_url = serve_embedded_assets().await;

    for (path, expected_type) in [
        ("/bundle.js", "text/javascript"),
        ("/styles.css", "text/css"),
    ] {
        let response = reqwest::get(format!("{base_url}{path}"))
            .await
            .unwrap_or_else(|e| panic!("request {path}: {e}"));
        assert!(response.status().is_success(), "{path} was not served");
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let body = response.text().await.expect("asset body");

        assert!(
            content_type.starts_with(expected_type),
            "{path} content type was {content_type}"
        );
        assert!(!body.is_empty(), "{path} was served empty");
    }
}
