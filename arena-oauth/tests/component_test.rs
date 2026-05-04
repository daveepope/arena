use arena::dependency::RunnableDependency;
use arena_oauth::{ensure_scopes, OauthDependency, TokenError};
use serde_json::Value;

fn init_test_logging() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .is_test(true)
        .try_init();
}

async fn start_oauth_https_default() -> OauthDependency {
    let dep = OauthDependency::builder("oauth https flow").build();
    assert!(
        dep.server_tls_certificate_pem().is_some(),
        "ephemeral oauth should expose server TLS certificate PEM before start"
    );
    let mut dep = dep;
    dep.start().await;
    dep
}

fn https_client_trusting_pem(pem: &str) -> reqwest::Client {
    let cert =
        reqwest::Certificate::from_pem(pem.as_bytes()).expect("parse server TLS certificate PEM");
    reqwest::Client::builder()
        .add_root_certificate(cert)
        .build()
        .expect("build reqwest client")
}

#[tokio::test]
async fn oauth_dependency_https_discovery_token_introspect_and_dependency_verify() {
    init_test_logging();
    let mut dep = start_oauth_https_default().await;

    let base = dep
        .base_url()
        .expect("base_url after start")
        .trim_end_matches('/')
        .to_string();
    assert!(
        base.starts_with("https://"),
        "expected https base_url, got {base}"
    );

    let pem = dep
        .server_tls_certificate_pem()
        .expect("server TLS PEM after ephemeral start");
    let client = https_client_trusting_pem(pem);

    let disc_url = format!("{base}/.well-known/oauth-authorization-server");
    let disc: Value = client
        .get(&disc_url)
        .send()
        .await
        .expect("discovery GET")
        .error_for_status()
        .expect("discovery status")
        .json()
        .await
        .expect("discovery JSON");

    let token_endpoint = disc["token_endpoint"]
        .as_str()
        .expect("token_endpoint string");
    let jwks_uri = disc["jwks_uri"].as_str().expect("jwks_uri string");
    let introspection_endpoint = disc["introspection_endpoint"]
        .as_str()
        .expect("introspection_endpoint string");

    assert!(token_endpoint.starts_with("https://"));
    assert!(jwks_uri.starts_with("https://"));
    assert!(introspection_endpoint.starts_with("https://"));

    let token_resp = client
        .post(token_endpoint)
        .form(&[("grant_type", "client_credentials")])
        .send()
        .await
        .expect("token POST")
        .error_for_status()
        .expect("token status");

    let token_json: Value = token_resp.json().await.expect("token JSON");
    let access_token = token_json["access_token"]
        .as_str()
        .expect("access_token string");

    dep.verify_access_token(access_token)
        .expect("dependency JWT verify");

    let intro: Value = client
        .post(introspection_endpoint)
        .form(&[("token", access_token)])
        .send()
        .await
        .expect("introspect POST")
        .error_for_status()
        .expect("introspect status")
        .json()
        .await
        .expect("introspect JSON");

    assert_eq!(intro["active"].as_bool(), Some(true));

    let pem_owned = pem.to_string();

    dep.stop().await;
    assert_eq!(
        dep.server_tls_certificate_pem(),
        Some(pem_owned.as_str()),
        "ephemeral TLS PEM must remain after stop() so clients keep the same trust anchor"
    );

    dep.start().await;
    assert_eq!(
        dep.server_tls_certificate_pem(),
        Some(pem_owned.as_str()),
        "ephemeral TLS PEM must not rotate on a subsequent start() of the same dependency"
    );

    let base_after = dep
        .base_url()
        .expect("base_url after second start")
        .trim_end_matches('/')
        .to_string();
    let disc_url_after = format!("{base_after}/.well-known/oauth-authorization-server");
    let _disc_after: Value = client
        .get(&disc_url_after)
        .send()
        .await
        .expect("discovery GET after restart")
        .error_for_status()
        .expect("discovery status after restart")
        .json()
        .await
        .expect("discovery JSON after restart");

    dep.stop().await;
}

#[tokio::test]
async fn oauth_dependency_issued_token_scope_claims_enforce_ensure_scopes() {
    init_test_logging();
    let mut dep = start_oauth_https_default().await;

    let base = dep
        .base_url()
        .expect("base_url after start")
        .trim_end_matches('/')
        .to_string();
    let pem = dep
        .server_tls_certificate_pem()
        .expect("server TLS PEM after ephemeral start");
    let client = https_client_trusting_pem(pem);

    let disc_url = format!("{base}/.well-known/oauth-authorization-server");
    let disc: Value = client
        .get(&disc_url)
        .send()
        .await
        .expect("discovery GET")
        .error_for_status()
        .expect("discovery status")
        .json()
        .await
        .expect("discovery JSON");

    let token_endpoint = disc["token_endpoint"]
        .as_str()
        .expect("token_endpoint string");

    let token_resp = client
        .post(token_endpoint)
        .form(&[
            ("grant_type", "client_credentials"),
            ("scope", "openid profile"),
        ])
        .send()
        .await
        .expect("token POST")
        .error_for_status()
        .expect("token status");

    let token_json: Value = token_resp.json().await.expect("token JSON");
    let access_token = token_json["access_token"]
        .as_str()
        .expect("access_token string");

    let claims = dep
        .verify_access_token(access_token)
        .expect("dependency JWT verify");

    ensure_scopes(&claims, &["openid"]).expect("openid is granted");
    let err = ensure_scopes(&claims, &["admin"]).expect_err("admin is not granted");
    assert!(
        matches!(err, TokenError::InsufficientScope { .. }),
        "expected InsufficientScope, got {err:?}"
    );

    let bare_resp = client
        .post(token_endpoint)
        .form(&[("grant_type", "client_credentials")])
        .send()
        .await
        .expect("token POST without scope")
        .error_for_status()
        .expect("token status");

    let bare_json: Value = bare_resp.json().await.expect("token JSON");
    let bare_token = bare_json["access_token"]
        .as_str()
        .expect("access_token string");
    let bare_claims = dep
        .verify_access_token(bare_token)
        .expect("bare token verifies");

    let missing = ensure_scopes(&bare_claims, &["openid"]).expect_err("scope claim absent");
    assert!(
        matches!(missing, TokenError::MissingScope),
        "expected MissingScope, got {missing:?}"
    );

    dep.stop().await;
}
