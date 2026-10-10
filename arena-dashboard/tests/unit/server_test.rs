use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use arena_dashboard::live_event_stream;
use futures::StreamExt;
use tokio::sync::broadcast;

#[tokio::test]
async fn live_event_stream_lagged_resyncs_to_latest_state() {
    let (tx, rx) = broadcast::channel::<String>(4);
    let last_state: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    for i in 0..10 {
        let document = format!(
            r#"{{"id":"lag-arena","state":"state-{i}","at":"2026-01-01T00:00:00.000Z"}}"#
        );
        last_state
            .lock()
            .unwrap()
            .insert("lag-arena".to_string(), document.clone());
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
    let last_state: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    let _ = tx.send("first".to_string());
    let _ = tx.send("second".to_string());

    let mut stream = Box::pin(live_event_stream(rx, last_state));
    let first = stream.next().await.expect("stream yields an event").expect("ok event");
    let second = stream.next().await.expect("stream yields an event").expect("ok event");

    assert!(format!("{first:?}").contains("first"));
    assert!(format!("{second:?}").contains("second"));
}
