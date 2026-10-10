use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::Router;
use futures::stream::{self, Stream, StreamExt};
use tokio::sync::broadcast;
use tower_http::services::ServeFile;
use tower_http::set_header::SetResponseHeaderLayer;

use crate::ingest;

const FALLBACK_INDEX_HTML: &str =
    "<!doctype html><title>Arena Dashboard</title><body>Web assets not found in runfiles - run via `bazel run //arena-dashboard:server`.</body>";

fn resolve_web_asset(name: &str) -> Option<PathBuf> {
    let runfiles = runfiles::Runfiles::create().ok()?;
    let path = runfiles.rlocation(format!("_main/arena-dashboard/web/{name}"))?;
    path.exists().then_some(path)
}

#[derive(Clone)]
pub struct AppState {
    tx: broadcast::Sender<String>,
    // Keyed by arena id so multiple arenas can be ingested and replayed to
    // new SSE connections concurrently, rather than one shared slot where a
    // second arena's events would overwrite the first's.
    last_state: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for AppState {
    fn default() -> Self {
        let (tx, _rx) = broadcast::channel(256);
        Self { tx, last_state: Arc::new(Mutex::new(HashMap::new())) }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn app(state: AppState) -> Router {
    let router = Router::new()
        .route("/ingest", post(ingest_handler))
        .route("/events", get(events))
        .with_state(state);

    let router = match resolve_web_asset("index.html") {
        Some(path) => router.route_service("/", ServeFile::new(path)),
        None => router.route("/", get(fallback_index)),
    };
    let router = match resolve_web_asset("bundle.js") {
        Some(path) => router.route_service("/bundle.js", ServeFile::new(path)),
        None => router,
    };
    let router = match resolve_web_asset("styles.css") {
        Some(path) => router.route_service("/styles.css", ServeFile::new(path)),
        None => router,
    };

    router.layer(SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    ))
}

async fn fallback_index() -> impl IntoResponse {
    Html(FALLBACK_INDEX_HTML)
}

async fn ingest_handler(State(state): State<AppState>, body: String) -> impl IntoResponse {
    match ingest::parse(&body) {
        Ok(parsed) => {
            state
                .last_state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(parsed.id, body.clone());
            let _ = state.tx.send(body);
            StatusCode::ACCEPTED
        }
        Err(_) => StatusCode::BAD_REQUEST,
    }
}

async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.tx.subscribe();

    let replay: Vec<String> = state
        .last_state
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .values()
        .cloned()
        .collect();
    let replay_stream = stream::iter(replay.into_iter().map(|data| Ok(Event::default().data(data))));

    let live_stream = live_event_stream(rx, Arc::clone(&state.last_state));

    Sse::new(replay_stream.chain(live_stream)).keep_alive(KeepAlive::default())
}

pub fn live_event_stream(
    rx: broadcast::Receiver<String>,
    last_state: Arc<Mutex<HashMap<String, String>>>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    stream::unfold(
        (rx, last_state, VecDeque::<String>::new()),
        |(mut rx, last_state, mut pending)| async move {
            loop {
                if let Some(data) = pending.pop_front() {
                    return Some((Ok(Event::default().data(data)), (rx, last_state, pending)));
                }
                match rx.recv().await {
                    Ok(data) => return Some((Ok(Event::default().data(data)), (rx, last_state, pending))),
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        pending = last_state
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .values()
                            .cloned()
                            .collect();
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        },
    )
}
