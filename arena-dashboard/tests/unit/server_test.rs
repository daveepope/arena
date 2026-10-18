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

async fn serve_dashboard() -> (String, AppState) {
    let state = AppState::new();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let served = state.clone();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app(served)).await;
    });
    (format!("http://{addr}"), state)
}

const SAMPLE_FOLDED: &str = "main;handler;compute 42\nmain;handler;idle 8\n";

async fn post_cpu_profile(base_url: &str, body: serde_json::Value) -> reqwest::StatusCode {
    reqwest::Client::new()
        .post(format!("{base_url}/profile/cpu"))
        .json(&body)
        .send()
        .await
        .expect("post cpu profile")
        .status()
}

#[tokio::test]
async fn post_cpu_profile_statewithoutfolded_isaccepted() {
    let (base_url, _state) = serve_dashboard().await;

    let status = post_cpu_profile(
        &base_url,
        serde_json::json!({"component": "svc", "state": "started"}),
    )
    .await;

    assert_eq!(status, reqwest::StatusCode::ACCEPTED);
}

#[tokio::test]
async fn post_cpu_profile_malformedbody_isrejected() {
    let (base_url, _state) = serve_dashboard().await;

    let status = reqwest::Client::new()
        .post(format!("{base_url}/profile/cpu"))
        .body("not json at all")
        .send()
        .await
        .expect("post cpu profile")
        .status();

    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_cpu_profile_unknowncomponent_returnsnotfound() {
    let (base_url, _state) = serve_dashboard().await;

    let response = reqwest::get(format!("{base_url}/profile/cpu/never-profiled"))
        .await
        .expect("get cpu profile");

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_cpu_profile_statereportedbutnosamples_returnsnotfound() {
    let (base_url, _state) = serve_dashboard().await;
    post_cpu_profile(
        &base_url,
        serde_json::json!({"component": "svc-pending", "state": "started"}),
    )
    .await;

    let response = reqwest::get(format!("{base_url}/profile/cpu/svc-pending"))
        .await
        .expect("get cpu profile");

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_cpu_profile_foldedstacksposted_returnssvgandhotspots() {
    let (base_url, _state) = serve_dashboard().await;
    post_cpu_profile(
        &base_url,
        serde_json::json!({"component": "svc-done", "state": "stopped", "folded": SAMPLE_FOLDED}),
    )
    .await;

    let body: serde_json::Value = reqwest::get(format!("{base_url}/profile/cpu/svc-done"))
        .await
        .expect("get cpu profile")
        .json()
        .await
        .expect("json body");

    assert!(body["svg"].as_str().expect("svg string").contains("<svg"));
    let hotspots = body["hotspots"].as_array().expect("hotspots array");
    assert_eq!(hotspots.len(), 2);
    assert_eq!(hotspots[0]["function"], "compute");
    assert_eq!(hotspots[0]["severity"], "critical");
}

#[tokio::test]
async fn get_cpu_profile_unrenderablestacks_returnsunprocessable() {
    let (base_url, _state) = serve_dashboard().await;
    post_cpu_profile(
        &base_url,
        serde_json::json!({"component": "svc-empty", "state": "stopped", "folded": ""}),
    )
    .await;

    let response = reqwest::get(format!("{base_url}/profile/cpu/svc-empty"))
        .await
        .expect("get cpu profile");

    assert_eq!(response.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn post_cpu_profile_samecomponenttwice_replacestheearliersamples() {
    let (base_url, _state) = serve_dashboard().await;
    post_cpu_profile(
        &base_url,
        serde_json::json!({"component": "svc-twice", "state": "stopped", "folded": "main;first 10\n"}),
    )
    .await;
    let first: serde_json::Value = reqwest::get(format!("{base_url}/profile/cpu/svc-twice"))
        .await
        .expect("get first")
        .json()
        .await
        .expect("json");
    assert_eq!(first["hotspots"][0]["function"], "first");

    post_cpu_profile(
        &base_url,
        serde_json::json!({"component": "svc-twice", "state": "stopped", "folded": "main;second 10\n"}),
    )
    .await;

    let second: serde_json::Value = reqwest::get(format!("{base_url}/profile/cpu/svc-twice"))
        .await
        .expect("get second")
        .json()
        .await
        .expect("json");
    assert_eq!(second["hotspots"][0]["function"], "second");
}

#[tokio::test]
async fn post_cpu_profile_anytransition_broadcastsatypedcpuprofileevent() {
    let (base_url, state) = serve_dashboard().await;
    let mut subscriber = state.subscribe();

    post_cpu_profile(
        &base_url,
        serde_json::json!({"component": "svc-broadcast", "state": "started"}),
    )
    .await;

    let message = tokio::time::timeout(std::time::Duration::from_secs(5), subscriber.recv())
        .await
        .expect("broadcast arrives")
        .expect("broadcast message");
    let event: serde_json::Value = serde_json::from_str(&message).expect("json event");
    assert_eq!(event["type"], "cpu_profile");
    assert_eq!(event["component"], "svc-broadcast");
    assert_eq!(event["state"], "started");
}

#[tokio::test]
async fn clear_stored_cpu_profile_discards_it() {
    let (base_url, _state) = serve_dashboard().await;
    post_cpu_profile(
        &base_url,
        serde_json::json!({"component": "svc-cleared", "state": "stopped", "folded": SAMPLE_FOLDED}),
    )
    .await;

    let cleared = reqwest::Client::new()
        .post(format!("{base_url}/clear"))
        .send()
        .await
        .expect("post clear")
        .status();
    let response = reqwest::get(format!("{base_url}/profile/cpu/svc-cleared"))
        .await
        .expect("get cpu profile");

    assert_eq!(cleared, reqwest::StatusCode::NO_CONTENT);
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_cpu_profile_rendered_declares_json_content_type() {
    let (base_url, _state) = serve_dashboard().await;
    post_cpu_profile(
        &base_url,
        serde_json::json!({"component": "svc-typed", "state": "stopped", "folded": SAMPLE_FOLDED}),
    )
    .await;

    let rendered = reqwest::get(format!("{base_url}/profile/cpu/svc-typed"))
        .await
        .expect("get cpu profile");
    let rendered_type = rendered.headers()[reqwest::header::CONTENT_TYPE].clone();
    let cached = reqwest::get(format!("{base_url}/profile/cpu/svc-typed"))
        .await
        .expect("get cached cpu profile");

    assert_eq!(rendered_type, "application/json");
    assert_eq!(
        cached.headers()[reqwest::header::CONTENT_TYPE],
        "application/json"
    );
}

#[tokio::test]
async fn get_cpu_profile_laterrunsamplednothing_returnsnotfound() {
    let (base_url, _state) = serve_dashboard().await;
    post_cpu_profile(
        &base_url,
        serde_json::json!({"component": "svc-restarted", "state": "stopped", "folded": SAMPLE_FOLDED}),
    )
    .await;

    post_cpu_profile(
        &base_url,
        serde_json::json!({"component": "svc-restarted", "state": "stopped"}),
    )
    .await;

    let response = reqwest::get(format!("{base_url}/profile/cpu/svc-restarted"))
        .await
        .expect("get cpu profile");

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
}

async fn post_scoped_playbook(
    base_url: &str,
    arena: &str,
    playbook: &str,
    body: serde_json::Value,
) -> reqwest::StatusCode {
    reqwest::Client::new()
        .post(format!("{base_url}/arena/{arena}/playbook/{playbook}"))
        .json(&body)
        .send()
        .await
        .expect("post scoped playbook")
        .status()
}

#[tokio::test]
async fn post_scoped_playbook_validpayload_isaccepted() {
    let (base_url, _state) = serve_dashboard().await;

    let status = post_scoped_playbook(
        &base_url,
        "arena-orders",
        "reset-orders",
        serde_json::json!({
            "arena_id": "arena-orders",
            "playbook_id": "reset-orders",
            "state": "started",
            "faults": []
        }),
    )
    .await;

    assert_eq!(status, reqwest::StatusCode::ACCEPTED);
}

#[tokio::test]
async fn post_scoped_playbook_malformedbody_isrejected() {
    let (base_url, _state) = serve_dashboard().await;

    let status = reqwest::Client::new()
        .post(format!("{base_url}/arena/arena-orders/playbook/reset-orders"))
        .body("not json")
        .send()
        .await
        .expect("post scoped playbook")
        .status();

    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_scoped_playbook_twoarenassameplaybookid_arekeptapart() {
    let (base_url, state) = serve_dashboard().await;
    let body = |arena: &str| {
        serde_json::json!({
            "arena_id": arena,
            "playbook_id": "reset-orders",
            "state": "started",
            "faults": []
        })
    };

    post_scoped_playbook(&base_url, "arena-orders", "reset-orders", body("arena-orders")).await;
    post_scoped_playbook(&base_url, "arena-catalog", "reset-orders", body("arena-catalog")).await;

    assert_eq!(state.scoped_playbook_count(), 2);
}

#[tokio::test]
async fn post_scoped_playbook_pathdisagreeingwithbody_isrejected() {
    let (base_url, _state) = serve_dashboard().await;

    let status = post_scoped_playbook(
        &base_url,
        "arena-orders",
        "reset-orders",
        serde_json::json!({
            "arena_id": "arena-catalog",
            "playbook_id": "reset-orders",
            "state": "started",
            "faults": []
        }),
    )
    .await;

    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_scoped_playbook_faults_reachthebrowserinfull() {
    let (base_url, state) = serve_dashboard().await;
    let mut events = state.subscribe();

    post_scoped_playbook(
        &base_url,
        "arena-orders",
        "reset-orders",
        serde_json::json!({
            "arena_id": "arena-orders",
            "playbook_id": "reset-orders",
            "state": "faulted",
            "faults": [{
                "id": "reset-orders",
                "subject": "playbook",
                "message": "seed data rejected",
                "at": "2026-10-18T00:00:00.000Z"
            }]
        }),
    )
    .await;

    let announcement = events.recv().await.expect("announcement");
    assert!(announcement.contains("seed data rejected"), "got: {announcement}");
}

#[tokio::test]
async fn clear_scopedplaybooksheld_areforgotten() {
    let (base_url, state) = serve_dashboard().await;
    post_scoped_playbook(
        &base_url,
        "arena-orders",
        "reset-orders",
        serde_json::json!({
            "arena_id": "arena-orders",
            "playbook_id": "reset-orders",
            "state": "started",
            "faults": []
        }),
    )
    .await;
    assert_eq!(state.scoped_playbook_count(), 1);

    reqwest::Client::new()
        .post(format!("{base_url}/clear"))
        .send()
        .await
        .expect("clear");

    assert_eq!(state.scoped_playbook_count(), 0);
}

#[tokio::test]
async fn post_scoped_playbook_beyondthelimit_isrefusedratherthangrowing() {
    let (base_url, state) = serve_dashboard().await;
    for index in 0..arena_dashboard::server::SCOPED_PLAYBOOK_LIMIT {
        let playbook = format!("playbook-{index}");
        post_scoped_playbook(
            &base_url,
            "arena-orders",
            &playbook,
            serde_json::json!({
                "arena_id": "arena-orders",
                "playbook_id": playbook,
                "state": "started",
                "faults": []
            }),
        )
        .await;
    }

    let status = post_scoped_playbook(
        &base_url,
        "arena-orders",
        "one-too-many",
        serde_json::json!({
            "arena_id": "arena-orders",
            "playbook_id": "one-too-many",
            "state": "started",
            "faults": []
        }),
    )
    .await;

    assert_eq!(status, reqwest::StatusCode::INSUFFICIENT_STORAGE);
    assert_eq!(
        state.scoped_playbook_count(),
        arena_dashboard::server::SCOPED_PLAYBOOK_LIMIT
    );
}

