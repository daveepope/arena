use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
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

pub const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; connect-src 'self'; img-src 'self' data:; \
style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'; frame-src 'self'; \
frame-ancestors 'none'; base-uri 'none'; form-action 'none'; object-src 'none'";

pub const CALLER_HEADER: &str = "x-arena-dashboard";
pub const CALLER_HEADER_VALUE: &str = "arena";
const LOOPBACK_HOSTS: [&str; 2] = ["127.0.0.1", "localhost"];

const RECONNECT_INTERVAL: Duration = Duration::from_millis(250);
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_millis(500);
pub const HISTORY_LIMIT_PER_ARENA: usize = 1000;
pub const PLAYBOOK_LIMIT: usize = 1000;
pub const ARENA_LIMIT: usize = 200;
pub const CPU_PROFILE_LIMIT: usize = 200;

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
        self.tx.send_replace(true);
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
    playbooks: Arc<Mutex<Vec<(String, String)>>>,
    playbook_history: Arc<Mutex<Vec<(String, Vec<String>)>>>,
    shutdown: Arc<ServerShutdown>,
}

#[derive(Clone, Default)]
struct CpuProfileRecord {
    state: String,
    folded: Option<String>,
    rendered: Option<String>,
    reason: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        let (tx, _rx) = broadcast::channel(256);
        Self {
            tx,
            last_state: Arc::new(Mutex::new(Vec::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            cpu_profiles: Arc::new(Mutex::new(Vec::new())),
            playbooks: Arc::new(Mutex::new(Vec::new())),
            playbook_history: Arc::new(Mutex::new(Vec::new())),
            shutdown: Arc::new(ServerShutdown::default()),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn playbook_count(&self) -> usize {
        self.playbooks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }

    pub fn shutdown_handle(&self) -> Arc<ServerShutdown> {
        Arc::clone(&self.shutdown)
    }
}

fn is_loopback_host(value: &str) -> bool {
    let without_port = match value.rsplit_once(':') {
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => host,
        _ => value,
    };
    let host = without_port.trim_start_matches('[').trim_end_matches(']');
    LOOPBACK_HOSTS.contains(&host) || host == "::1"
}

pub fn addresses_this_server(request: &Request) -> bool {
    let headers = request.headers();
    let host_is_loopback = match headers.get(header::HOST).and_then(|h| h.to_str().ok()) {
        Some(host) => is_loopback_host(host),
        None => true,
    };
    if !host_is_loopback {
        return false;
    }
    match headers.get(header::ORIGIN).and_then(|h| h.to_str().ok()) {
        Some(origin) => origin
            .split("//")
            .nth(1)
            .is_some_and(is_loopback_host),
        None => true,
    }
}

pub fn carries_caller_header(request: &Request) -> bool {
    request
        .headers()
        .get(CALLER_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == CALLER_HEADER_VALUE)
}

async fn guard_caller(request: Request, next: Next) -> axum::response::Response {
    if !addresses_this_server(&request) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if request.method() == Method::POST && !carries_caller_header(&request) {
        return StatusCode::FORBIDDEN.into_response();
    }
    next.run(request).await
}

pub fn app(state: AppState) -> Router {
    let router = Router::new()
        .route("/ingest", post(ingest_handler))
        .route("/events", get(events))
        .route("/clear", post(clear_handler))
        .route("/history", get(history_handler))
        .route("/shutdown", post(shutdown_handler))
        .route("/profile/cpu", post(cpu_profile_ingest_handler))
        .route("/profiles", get(cpu_profiles_handler))
        .route("/profile/cpu/{component}", get(cpu_profile_handler))
        .route(
            "/arena/{arena}/playbook/{playbook}",
            post(playbook_ingest_handler),
        )
        .with_state(state);

    let router = router
        .route("/", get(index_handler))
        .route("/bundle.js", get(bundle_handler))
        .route("/styles.css", get(styles_handler));

    router
        .layer(middleware::from_fn(guard_caller))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CONTENT_SECURITY_POLICY),
        ))
        .layer(SetResponseHeaderLayer::overriding(
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
    let playbook_history = state
        .playbook_history
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let history = state.history.lock().unwrap_or_else(|e| e.into_inner());
    let arenas: Vec<serde_json::Value> = history
        .iter()
        .map(|(id, bodies)| {
            let states: Vec<serde_json::Value> = bodies
                .iter()
                .filter_map(|body| serde_json::from_str(body).ok())
                .collect();
            serde_json::json!({
                "id": id,
                "states": states,
                "playbooks": recorded_playbooks(&playbook_history, id),
            })
        })
        .collect();
    axum::Json(arenas)
}

fn recorded_playbooks(
    playbook_history: &[(String, Vec<String>)],
    arena_id: &str,
) -> Vec<serde_json::Value> {
    playbook_history
        .iter()
        .find(|(id, _)| id == arena_id)
        .map(|(_, bodies)| {
            bodies
                .iter()
                .filter_map(|body| serde_json::from_str(body).ok())
                .collect()
        })
        .unwrap_or_default()
}

async fn clear_handler(State(state): State<AppState>) -> impl IntoResponse {
    state.last_state.lock().unwrap_or_else(|e| e.into_inner()).clear();
    state.history.lock().unwrap_or_else(|e| e.into_inner()).clear();
    state.cpu_profiles.lock().unwrap_or_else(|e| e.into_inner()).clear();
    state
        .playbooks
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
    state
        .playbook_history
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
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
                None => {
                    if last_state.len() >= ARENA_LIMIT {
                        tracing::error!(
                            limit = ARENA_LIMIT,
                            arena = %parsed_id,
                            "arena-dashboard is holding its limit of arenas; raise ARENA_LIMIT to hold more"
                        );
                        return StatusCode::INSUFFICIENT_STORAGE;
                    }
                    last_state.push((parsed_id.clone(), body.clone()))
                }
            }
            drop(last_state);

            let mut history = state.history.lock().unwrap_or_else(|e| e.into_inner());
            let recorded = record_history(&mut history, &parsed_id, body.clone());
            drop(history);
            if !recorded {
                return StatusCode::INSUFFICIENT_STORAGE;
            }

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

const PROFILE_TERMINAL_STATES: [&str; 2] = ["stopped", "faulted"];

async fn playbook_ingest_handler(
    State(state): State<AppState>,
    axum::extract::Path((arena, playbook)): axum::extract::Path<(String, String)>,
    body: String,
) -> impl IntoResponse {
    let Ok(payload) = ingest::parse_playbook(&body) else {
        return StatusCode::BAD_REQUEST;
    };
    if payload.arena_id != arena || payload.id != playbook {
        tracing::warn!(
            path_arena = %arena,
            path_playbook = %playbook,
            body_arena = %payload.arena_id,
            body_playbook = %payload.id,
            "playbook update disagrees with its own path"
        );
        return StatusCode::BAD_REQUEST;
    }

    let record = serde_json::json!({
        "arena": arena,
        "playbook": playbook,
        "playbook_type": payload.playbook_type,
        "kind": payload.kind,
        "state": payload.state,
        "runs": payload.runs,
        "at": payload.at,
        "faults": payload.faults,
    });
    let recorded = record.to_string();
    let mut announced = record;
    announced["type"] = serde_json::Value::String("playbook".to_string());
    let announcement = announced.to_string();

    {
        let mut playbooks = state
            .playbooks
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let key = (arena.clone(), playbook.clone());
        if !playbooks.contains(&key) {
            if playbooks.len() >= PLAYBOOK_LIMIT {
                tracing::error!(
                    limit = PLAYBOOK_LIMIT,
                    arena = %arena,
                    playbook = %playbook,
                    "arena-dashboard is holding its limit of playbooks; raise PLAYBOOK_LIMIT to hold more"
                );
                return StatusCode::INSUFFICIENT_STORAGE;
            }
            playbooks.push(key);
        }
    }

    {
        let mut playbook_history = state
            .playbook_history
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if !record_history(&mut playbook_history, &arena, recorded) {
            return StatusCode::INSUFFICIENT_STORAGE;
        }
    }

    let _ = state.tx.send(announcement);
    StatusCode::ACCEPTED
}

async fn cpu_profiles_handler(State(state): State<AppState>) -> impl IntoResponse {
    let held: Vec<serde_json::Value> = state
        .cpu_profiles
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .map(|(component, record)| {
            serde_json::json!({
                "component": component,
                "state": record.state,
                "report": record.folded.is_some() || record.rendered.is_some(),
                "reason": record.reason,
            })
        })
        .collect();
    json_response(StatusCode::OK, serde_json::Value::Array(held).to_string())
}

async fn cpu_profile_ingest_handler(
    State(state): State<AppState>,
    body: String,
) -> impl IntoResponse {
    let Ok(payload) = serde_json::from_str::<ingest::CpuProfilePayload>(&body) else {
        return StatusCode::BAD_REQUEST;
    };

    let report_ready;
    {
        let mut profiles = state.cpu_profiles.lock().unwrap_or_else(|e| e.into_inner());
        let record = match profiles.iter_mut().find(|(id, _)| *id == payload.component) {
            Some(entry) => &mut entry.1,
            None => {
                if profiles.len() >= CPU_PROFILE_LIMIT {
                    tracing::error!(
                        limit = CPU_PROFILE_LIMIT,
                        component = %payload.component,
                        "arena-dashboard is holding its limit of cpu profiles; raise CPU_PROFILE_LIMIT to hold more"
                    );
                    return StatusCode::INSUFFICIENT_STORAGE;
                }
                profiles.push((payload.component.clone(), CpuProfileRecord::default()));
                &mut profiles.last_mut().expect("just pushed").1
            }
        };
        record.state = payload.state.clone();
        record.reason = payload.reason.clone();
        match &payload.folded {
            Some(folded) => {
                record.folded = Some(folded.clone());
                record.rendered = None;
            }
            None if PROFILE_TERMINAL_STATES.contains(&payload.state.as_str()) => {
                record.folded = None;
                record.rendered = None;
            }
            None => {}
        }
        report_ready = record.folded.is_some();
    }

    let announcement = serde_json::json!({
        "type": "cpu_profile",
        "component": payload.component,
        "state": payload.state,
        "report": report_ready,
        "reason": payload.reason,
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
                "application_hotspots": profile.application_hotspots,
                "runtime_frames": arena_profile::render::RUNTIME_FRAMES
                    .iter()
                    .map(|group| {
                        let mut frames: Vec<&str> = group.starts_with.to_vec();
                        frames.extend(group.contains.iter().copied());
                        serde_json::json!({ "language": group.language, "frames": frames })
                    })
                    .collect::<Vec<serde_json::Value>>(),
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

pub fn record_history(
    history: &mut Vec<(String, Vec<String>)>,
    arena_id: &str,
    body: String,
) -> bool {
    match history.iter_mut().find(|(id, _)| id == arena_id) {
        Some(entry) => {
            if entry.1.len() >= HISTORY_LIMIT_PER_ARENA {
                tracing::error!(
                    limit = HISTORY_LIMIT_PER_ARENA,
                    arena = %arena_id,
                    "arena-dashboard is holding its limit of reports for this arena; raise HISTORY_LIMIT_PER_ARENA to hold more"
                );
                return false;
            }
            entry.1.push(body);
            true
        }
        None => {
            if history.len() >= ARENA_LIMIT {
                tracing::error!(
                    limit = ARENA_LIMIT,
                    arena = %arena_id,
                    "arena-dashboard is holding its limit of arenas; raise ARENA_LIMIT to hold more"
                );
                return false;
            }
            history.push((arena_id.to_string(), vec![body]));
            true
        }
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
