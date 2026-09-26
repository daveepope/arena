use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use serde_json::json;

use crate::oauth_common::{introspection_active, IntrospectForm, OAuthSigningState, TokenForm};
use crate::token::{issue_access_token, verify_access_token};

pub(crate) const RESERVED_PATHS: [&str; 4] = [
    "/.well-known/oauth-authorization-server",
    "/.well-known/openid-configuration",
    "/oauth/token",
    "/oauth/introspect",
];

pub(crate) fn https_router(state: Arc<OAuthSigningState>) -> Router {
    let mut router = Router::new()
        .route(RESERVED_PATHS[0], get(get_authorization_server_metadata))
        .route(RESERVED_PATHS[1], get(get_openid_configuration))
        .route(RESERVED_PATHS[2], post(post_token))
        .route(RESERVED_PATHS[3], post(post_introspect));

    for idx in 0..state.issuers.len() {
        let jwks_path = state.issuers[idx].jwks_path.clone();
        router = router.route(
            &jwks_path,
            get(move |State(s): State<Arc<OAuthSigningState>>| get_jwks_for_issuer(s, idx)),
        );
    }

    router.with_state(state)
}

async fn get_authorization_server_metadata(
    State(s): State<Arc<OAuthSigningState>>,
) -> Json<crate::discovery::OAuthAuthorizationServerMetadata> {
    Json((*s.metadata).clone())
}

async fn get_openid_configuration(
    State(s): State<Arc<OAuthSigningState>>,
) -> Json<crate::discovery::OAuthAuthorizationServerMetadata> {
    Json((*s.metadata).clone())
}

async fn get_jwks_for_issuer(s: Arc<OAuthSigningState>, idx: usize) -> Json<serde_json::Value> {
    Json(s.issuers[idx].keys.resolve().jwks_json())
}

async fn post_token(
    State(s): State<Arc<OAuthSigningState>>,
    Form(form): Form<TokenForm>,
) -> impl IntoResponse {
    if form.grant_type != "client_credentials" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "unsupported_grant_type" })),
        )
            .into_response();
    }
    let scopes: Vec<String> = form
        .scope
        .as_deref()
        .unwrap_or("")
        .split_whitespace()
        .map(String::from)
        .filter(|x| !x.is_empty())
        .collect();
    let sub = form
        .client_id
        .clone()
        .unwrap_or_else(|| "client".to_string());
    let access_token = match issue_access_token(
        s.default_issuer().keys.resolve(),
        &s.metadata.issuer,
        &sub,
        &scopes,
        s.token_ttl_secs,
    ) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, op = "oauth_issue_token", "token issue failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "server_error" })),
            )
                .into_response();
        }
    };
    Json(json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": s.token_ttl_secs,
        "scope": form.scope.unwrap_or_default(),
    }))
    .into_response()
}

async fn post_introspect(
    State(s): State<Arc<OAuthSigningState>>,
    Form(form): Form<IntrospectForm>,
) -> impl IntoResponse {
    for issuer in &s.issuers {
        let issuer_string = s.issuer_string(issuer);
        if let Ok(claims) = verify_access_token(&form.token, issuer.keys.resolve(), &issuer_string) {
            return Json(introspection_active(&claims)).into_response();
        }
    }
    Json(json!({ "active": false })).into_response()
}
