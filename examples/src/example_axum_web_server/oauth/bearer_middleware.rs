use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

use super::super::state::AppState;

fn bearer_token(header_value: &str) -> Option<&str> {
    let prefix = "Bearer ";
    if header_value.len() > prefix.len()
        && header_value[..prefix.len()].eq_ignore_ascii_case(prefix)
    {
        Some(header_value[prefix.len()..].trim())
    } else {
        None
    }
}

pub async fn oauth_bearer_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    if req.uri().path() == "/health" {
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "missing Authorization header".into(),
        ))?;

    let token = bearer_token(auth_header).ok_or((
        StatusCode::UNAUTHORIZED,
        "expected Authorization: Bearer <token>".into(),
    ))?;

    let claims = state
        .jwt
        .verify_access_token(token)
        .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

    req.extensions_mut().insert(claims.clone());
    Ok(next.run(req).await)
}
