use std::time::Duration;

use arena_dashboard::{app, AppState};
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
async fn ingest_malformedbody_returnsbadrequest() {
    let base_url = spawn_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base_url}/ingest"))
        .body("not json")
        .send()
        .await
        .expect("post ingest");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn root_noassets_servesfallbackhtml() {
    let base_url = spawn_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(base_url)
        .send()
        .await
        .expect("request root");

    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("read body");
    assert!(body.contains("Arena Dashboard"));
}

#[tokio::test]
async fn staticasset_noassets_returnsnotfound() {
    let base_url = spawn_server().await;
    let client = reqwest::Client::new();

    let bundle_response = client
        .get(format!("{base_url}/bundle.js"))
        .send()
        .await
        .expect("request bundle.js");
    let styles_response = client
        .get(format!("{base_url}/styles.css"))
        .send()
        .await
        .expect("request styles.css");

    assert_eq!(bundle_response.status(), 404);
    assert_eq!(styles_response.status(), 404);
}
