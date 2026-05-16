use std::time::Instant;

use axum::body::Body;
use axum::http::Request;
use axum::middleware;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::get;
use axum::Router;

use super::oauth;
use super::readings::{create_reading, list_readings};
use super::state::AppState;

async fn health() -> &'static str {
    "ok"
}

async fn log_requests(req: Request<Body>, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let sw = Instant::now();

    let res = next.run(req).await;

    tracing::debug!(
        http_method = %method,
        http_uri = %uri,
        status = %res.status(),
        elapsed = ?sw.elapsed(),
        phase = "http_request_finished",
        "request completed",
    );
    res
}

pub fn build_router(state: AppState) -> Router {
    let jwt_state = state.clone();
    Router::new()
        .route("/health", get(health))
        .route("/readings", get(list_readings).post(create_reading))
        .layer(middleware::from_fn_with_state(
            jwt_state,
            oauth::oauth_bearer_middleware,
        ))
        .layer(middleware::from_fn(log_requests))
        .with_state(state)
}
