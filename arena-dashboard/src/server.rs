use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::State;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::Router;
use futures::stream::{self, Stream};
use futures::StreamExt;
use tokio::sync::{broadcast, watch};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::ingest;

#[cfg(arena_embedded_web_assets)]
const INDEX_HTML: &str = include_str!(env!("ARENA_DASHBOARD_INDEX_HTML"));
#[cfg(arena_embedded_web_assets)]
const BUNDLE_JS: &str = include_str!(env!("ARENA_DASHBOARD_BUNDLE_JS"));
#[cfg(arena_embedded_web_assets)]
const STYLES_CSS: &str = include_str!(env!("ARENA_DASHBOARD_STYLES_CSS"));

#[cfg(not(arena_embedded_web_assets))]
const INDEX_HTML: &str =
    "<!doctype html><title>Arena Topology Dashboard</title><body>This build has no web assets embedded.</body>";
#[cfg(not(arena_embedded_web_assets))]
const BUNDLE_JS: &str = "";
#[cfg(not(arena_embedded_web_assets))]
const STYLES_CSS: &str = "";

const RECONNECT_INTERVAL: Duration = Duration::from_millis(250);
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_millis(500);
pub const HISTORY_LIMIT_PER_ARENA: usize = 1000;

pub struct ServerShutdown {
    tx: watch::Sender<bool>,
}

impl Default for ServerShutdown {
    fn default() -> Self {
        Self {
            tx: watch::channel(false).0,
        }
    }
}

impl ServerShutdown {
    pub fn trigger(&self) {
        let _ = self.tx.send(true);
    }

    pub async fn wait(&self) {
        let mut rx = self.tx.subscribe();
        if *rx.borrow_and_update() {
            return;
        }
        let _ = rx.changed().await;
    }
}

#[derive(Clone)]
pub struct AppState {
    tx: broadcast::Sender<String>,
    last_state: Arc<Mutex<Vec<(String, String)>>>,
    history: Arc<Mutex<Vec<(String, Vec<String>)>>>,
    cpu_profiles: Arc<Mutex<Vec<(String, CpuProfileRecord)>>>,
    shutdown: Arc<ServerShutdown>,
}

#[derive(Clone, Default)]
struct CpuProfileRecord {
    folded: Option<String>,
    rendered: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        let (tx, _rx) = broadcast::channel(256);
        Self {
            tx,
            last_state: Arc::new(Mutex::new(Vec::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            cpu_profiles: Arc::new(Mutex::new(Vec::new())),
            shutdown: Arc::new(ServerShutdown::default()),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }

    pub fn shutdown_handle(&self) -> Arc<ServerShutdown> {
        Arc::clone(&self.shutdown)
    }
}

pub fn app(state: AppState) -> Router {
    let router = Router::new()
        .route("/ingest", post(ingest_handler))
        .route("/events", get(events))
        .route("/clear", post(clear_handler))
        .route("/history", get(history_handler))
        .route("/shutdown", post(shutdown_handler))
        .route("/profile/cpu", post(cpu_profile_ingest_handler))
        .route("/profile/cpu/{component}", get(cpu_profile_handler))
        .with_state(state);

    let router = router
        .route("/", get(index_handler))
        .route("/bundle.js", get(bundle_handler))
        .route("/styles.css", get(styles_handler));

    router.layer(SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    ))
}

async fn index_handler() -> impl IntoResponse {
    Html(INDEX_HTML)
}

async fn bundle_handler() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        BUNDLE_JS,
    )
}

async fn styles_handler() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], STYLES_CSS)
}

async fn history_handler(State(state): State<AppState>) -> impl IntoResponse {
    let history = state.history.lock().unwrap_or_else(|e| e.into_inner());
    let arenas: Vec<serde_json::Value> = history
        .iter()
        .map(|(id, bodies)| {
            let states: Vec<serde_json::Value> = bodies
                .iter()
                .filter_map(|body| serde_json::from_str(body).ok())
                .collect();
            serde_json::json!({ "id": id, "states": states })
        })
        .collect();
    axum::Json(arenas)
}

async fn clear_handler(State(state): State<AppState>) -> impl IntoResponse {
    state.last_state.lock().unwrap_or_else(|e| e.into_inner()).clear();
    state.history.lock().unwrap_or_else(|e| e.into_inner()).clear();
    state.cpu_profiles.lock().unwrap_or_else(|e| e.into_inner()).clear();
    let _ = state.tx.send(r#"{"type":"clear"}"#.to_string());
    StatusCode::NO_CONTENT
}

async fn shutdown_handler(State(state): State<AppState>) -> impl IntoResponse {
    state.shutdown.trigger();
    StatusCode::ACCEPTED
}

async fn ingest_handler(State(state): State<AppState>, body: String) -> impl IntoResponse {
    match ingest::parse(&body) {
        Ok(parsed) => {
            let parsed_id = parsed.id;
            let mut last_state = state.last_state.lock().unwrap_or_else(|e| e.into_inner());
            match last_state.iter_mut().find(|(id, _)| *id == parsed_id) {
                Some(entry) => entry.1 = body.clone(),
                None => last_state.push((parsed_id.clone(), body.clone())),
            }
            drop(last_state);

            let mut history = state.history.lock().unwrap_or_else(|e| e.into_inner());
            record_history(&mut history, &parsed_id, body.clone());
            drop(history);

            let _ = state.tx.send(body);
            StatusCode::ACCEPTED
        }
        Err(_) => StatusCode::BAD_REQUEST,
    }
}

fn json_response(status: StatusCode, body: String) -> axum::response::Response {
    (
        status,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        )],
        body,
    )
        .into_response()
}

async fn cpu_profile_ingest_handler(
    State(state): State<AppState>,
    body: String,
) -> impl IntoResponse {
    let Ok(payload) = serde_json::from_str::<ingest::CpuProfilePayload>(&body) else {
        return StatusCode::BAD_REQUEST;
    };

    {
        let mut profiles = state.cpu_profiles.lock().unwrap_or_else(|e| e.into_inner());
        let record = match profiles.iter_mut().find(|(id, _)| *id == payload.component) {
            Some(entry) => &mut entry.1,
            None => {
                profiles.push((payload.component.clone(), CpuProfileRecord::default()));
                &mut profiles.last_mut().expect("just pushed").1
            }
        };
        if let Some(folded) = &payload.folded {
            record.folded = Some(folded.clone());
            record.rendered = None;
        }
    }

    let announcement = serde_json::json!({
        "type": "cpu_profile",
        "component": payload.component,
        "state": payload.state,
    });
    let _ = state.tx.send(announcement.to_string());
    StatusCode::ACCEPTED
}

async fn cpu_profile_handler(
    State(state): State<AppState>,
    axum::extract::Path(component): axum::extract::Path<String>,
) -> impl IntoResponse {
    let cached = {
        let profiles = state.cpu_profiles.lock().unwrap_or_else(|e| e.into_inner());
        profiles
            .iter()
            .find(|(id, _)| *id == component)
            .map(|(_, record)| record.clone())
    };

    let Some(record) = cached else {
        return (StatusCode::NOT_FOUND, axum::Json(serde_json::Value::Null)).into_response();
    };
    if let Some(rendered) = record.rendered {
        return json_response(StatusCode::OK, rendered);
    }
    let Some(folded) = record.folded else {
        return (StatusCode::NOT_FOUND, axum::Json(serde_json::Value::Null)).into_response();
    };

    match arena_profile::render_folded(&folded) {
        Ok(profile) => {
            let body = serde_json::json!({
                "svg": profile.svg,
                "hotspots": profile.hotspots,
            })
            .to_string();
            let mut profiles = state.cpu_profiles.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = profiles.iter_mut().find(|(id, _)| *id == component) {
                entry.1.rendered = Some(body.clone());
            }
            drop(profiles);
            json_response(StatusCode::OK, body)
        }
        Err(e) => {
            tracing::warn!(component = %component, error = %e, "failed to render cpu profile");
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                axum::Json(serde_json::Value::Null),
            )
                .into_response()
        }
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
        .iter()
        .map(|(_, body)| body.clone())
        .collect();
    let replay_stream = stream::iter(
        replay
            .into_iter()
            .map(|data| Ok(Event::default().data(data).retry(RECONNECT_INTERVAL))),
    );

    let live_stream = live_event_stream(rx, Arc::clone(&state.last_state));
    let shutdown = state.shutdown_handle();

    Sse::new(
        replay_stream
            .chain(live_stream)
            .take_until(async move { shutdown.wait().await }),
    )
        .keep_alive(
            KeepAlive::new()
                .interval(KEEP_ALIVE_INTERVAL)
                .event(Event::default().event("heartbeat").data("alive").retry(RECONNECT_INTERVAL)),
        )
}

pub fn record_history(history: &mut Vec<(String, Vec<String>)>, arena_id: &str, body: String) {
    match history.iter_mut().find(|(id, _)| id == arena_id) {
        Some(entry) => {
            entry.1.push(body);
            if entry.1.len() > HISTORY_LIMIT_PER_ARENA {
                let excess = entry.1.len() - HISTORY_LIMIT_PER_ARENA;
                entry.1.drain(0..excess);
            }
        }
        None => history.push((arena_id.to_string(), vec![body])),
    }
}

pub fn live_event_stream(
    rx: broadcast::Receiver<String>,
    last_state: Arc<Mutex<Vec<(String, String)>>>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    stream::unfold(
        (rx, last_state, VecDeque::<String>::new()),
        |(mut rx, last_state, mut pending)| async move {
            loop {
                if let Some(data) = pending.pop_front() {
                    return Some((
                        Ok(Event::default().data(data).retry(RECONNECT_INTERVAL)),
                        (rx, last_state, pending),
                    ));
                }
                match rx.recv().await {
                    Ok(data) => {
                        return Some((
                            Ok(Event::default().data(data).retry(RECONNECT_INTERVAL)),
                            (rx, last_state, pending),
                        ))
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        pending = last_state
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .iter()
                            .map(|(_, body)| body.clone())
                            .collect();
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        },
    )
}
