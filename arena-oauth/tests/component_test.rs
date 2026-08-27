use arena::dependency::RunnableDependency;
use arena_oauth::{ensure_scopes, IssuerConfig, OauthDependency, Provider, TokenError};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::RsaPrivateKey;
use serde_json::{json, Value};

fn init_test_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
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
async fn oauth_dependency_http_transport_serves_discovery_token_and_verifies_jwt() {
    init_test_logging();
    let mut dep = OauthDependency::builder("oauth http flow").with_http().build();
    assert!(
        dep.server_tls_certificate_pem().is_none(),
        "http transport should not expose any server TLS certificate PEM"
    );
    dep.start().await;

    let base = dep
        .base_url()
        .expect("base_url after start")
        .trim_end_matches('/')
        .to_string();
    assert!(
        base.starts_with("http://"),
        "expected plain http base_url, got {base}"
    );

    let client = reqwest::Client::new();

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
    assert!(token_endpoint.starts_with("http://"));
    assert!(jwks_uri.starts_with("http://"));

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

fn decoding_key_for(dep: &OauthDependency, index: usize) -> jsonwebtoken::DecodingKey {
    let pem = dep.signing_key_pem(index).expect("signing key pem");
    let private_key = RsaPrivateKey::from_pkcs8_pem(&pem).expect("parse pkcs8 pem");
    let public_pem = private_key
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .expect("public key pem");
    jsonwebtoken::DecodingKey::from_rsa_pem(public_pem.as_bytes()).expect("build decoding key")
}

fn unpublished_key_token(claims: &Value) -> String {
    let mut rng = rand::thread_rng();
    let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("generate rsa key");
    let pem = private_key
        .to_pkcs8_pem(LineEnding::LF)
        .expect("encode pkcs8 pem");
    let encoding_key = EncodingKey::from_rsa_pem(pem.as_bytes()).expect("build encoding key");
    encode(&Header::new(Algorithm::RS256), claims, &encoding_key).expect("sign with unpublished key")
}

#[tokio::test(flavor = "multi_thread")]
async fn oauth_dependency_with_cognito_and_okta_issuers_reproduces_mixed_topology_use_case() {
    init_test_logging();
    let mut dep = OauthDependency::builder("oauth cognito okta mixed")
        .with_http()
        .with_provider(Provider::Cognito {
            pool_id: "us-east-1_abc123".to_string(),
        })
        .with_provider(Provider::Okta)
        .build();
    dep.start().await;

    let base = dep
        .base_url()
        .expect("base_url after start")
        .trim_end_matches('/')
        .to_string();
    let client = reqwest::Client::new();

    let cognito_jwks: Value = client
        .get(format!("{base}/us-east-1_abc123/.well-known/jwks.json"))
        .send()
        .await
        .expect("cognito jwks GET")
        .error_for_status()
        .expect("cognito jwks status")
        .json()
        .await
        .expect("cognito jwks JSON");
    let okta_jwks: Value = client
        .get(format!("{base}/v1/keys"))
        .send()
        .await
        .expect("okta jwks GET")
        .error_for_status()
        .expect("okta jwks status")
        .json()
        .await
        .expect("okta jwks JSON");
    let cognito_kid = cognito_jwks["keys"][0]["kid"].as_str().expect("cognito kid");
    let okta_kid = okta_jwks["keys"][0]["kid"].as_str().expect("okta kid");
    assert_ne!(
        cognito_kid, okta_kid,
        "each issuer must publish a distinct signing key"
    );

    let cognito_issuer = dep.issuer_at(0).expect("cognito issuer");
    let okta_issuer = dep.issuer_at(1).expect("okta issuer");
    assert_eq!(cognito_issuer, format!("{base}/us-east-1_abc123"));
    assert_eq!(okta_issuer, base);

    let cognito_claims = json!({
        "iss": cognito_issuer,
        "sub": "test-subject",
        "exp": 9_999_999_999u64,
        "iat": 0,
    });
    let cognito_token = dep
        .sign_claims(0, &cognito_claims)
        .expect("sign cognito token");
    dep.verify_access_token(&cognito_token)
        .expect("dependency verify_access_token succeeds for the default issuer's token");

    let okta_claims = json!({
        "iss": okta_issuer,
        "sub": "test-subject",
        "exp": 9_999_999_999u64,
        "iat": 0,
    });
    let okta_token = dep.sign_claims(1, &okta_claims).expect("sign okta token");
    dep.verify_access_token(&okta_token).expect(
        "dependency verify_access_token succeeds for a non-default (Okta) issuer's token",
    );

    let mut cognito_validation = jsonwebtoken::Validation::new(Algorithm::RS256);
    cognito_validation.set_issuer(&[cognito_issuer.as_str()]);
    jsonwebtoken::decode::<Value>(
        &cognito_token,
        &decoding_key_for(&dep, 0),
        &cognito_validation,
    )
    .expect("cognito token verifies against cognito's own key");

    let mut okta_validation = jsonwebtoken::Validation::new(Algorithm::RS256);
    okta_validation.set_issuer(&[cognito_issuer.as_str()]);
    jsonwebtoken::decode::<Value>(&cognito_token, &decoding_key_for(&dep, 1), &okta_validation)
        .expect_err("cognito token must not verify against okta's key");

    let forged_claims = json!({
        "iss": cognito_issuer,
        "sub": "attacker",
        "exp": 9_999_999_999u64,
        "iat": 0,
    });
    let forged_token = unpublished_key_token(&forged_claims);
    jsonwebtoken::decode::<Value>(
        &forged_token,
        &decoding_key_for(&dep, 0),
        &cognito_validation,
    )
    .expect_err("token signed by an unpublished key must not verify against cognito's key");
    jsonwebtoken::decode::<Value>(&forged_token, &decoding_key_for(&dep, 1), &okta_validation)
        .expect_err("token signed by an unpublished key must not verify against okta's key");
    dep.verify_access_token(&forged_token)
        .expect_err("dependency verify_access_token must reject a token signed by an unpublished key against every registered issuer");

    let introspect_url = format!("{base}/oauth/introspect");
    let okta_introspect: Value = client
        .post(&introspect_url)
        .form(&[("token", okta_token.as_str())])
        .send()
        .await
        .expect("okta introspect POST")
        .error_for_status()
        .expect("okta introspect status")
        .json()
        .await
        .expect("okta introspect JSON");
    assert_eq!(
        okta_introspect["active"].as_bool(),
        Some(true),
        "introspection must recognize a valid non-default (Okta) issuer's token as active"
    );

    let forged_introspect: Value = client
        .post(&introspect_url)
        .form(&[("token", forged_token.as_str())])
        .send()
        .await
        .expect("forged introspect POST")
        .error_for_status()
        .expect("forged introspect status")
        .json()
        .await
        .expect("forged introspect JSON");
    assert_eq!(forged_introspect["active"].as_bool(), Some(false));

    dep.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn oauth_dependency_serves_jwks_and_verifiable_tokens_across_all_providers() {
    init_test_logging();

    enum IssuerSource {
        Provider(Provider),
        Custom(IssuerConfig),
    }

    struct Case {
        name: &'static str,
        source: IssuerSource,
        expected_issuer_path: &'static str,
        expected_jwks_path: &'static str,
    }

    let cases = vec![
        Case {
            name: "cognito",
            source: IssuerSource::Provider(Provider::Cognito {
                pool_id: "pool-x".to_string(),
            }),
            expected_issuer_path: "/pool-x",
            expected_jwks_path: "/pool-x/.well-known/jwks.json",
        },
        Case {
            name: "okta",
            source: IssuerSource::Provider(Provider::Okta),
            expected_issuer_path: "",
            expected_jwks_path: "/v1/keys",
        },
        Case {
            name: "entra_id",
            source: IssuerSource::Provider(Provider::EntraId {
                tenant_id: "tenant-x".to_string(),
            }),
            expected_issuer_path: "/tenant-x/v2.0",
            expected_jwks_path: "/tenant-x/discovery/v2.0/keys",
        },
        Case {
            name: "custom",
            source: IssuerSource::Custom(
                IssuerConfig::new()
                    .with_issuer_path("/custom")
                    .with_jwks_path("/custom/keys"),
            ),
            expected_issuer_path: "/custom",
            expected_jwks_path: "/custom/keys",
        },
    ];

    for case in cases {
        let builder = OauthDependency::builder(format!("oauth provider matrix {}", case.name))
            .with_http();
        let builder = match case.source {
            IssuerSource::Provider(provider) => builder.with_provider(provider),
            IssuerSource::Custom(config) => builder.with_issuer(config),
        };
        let mut dep = builder.build();
        dep.start().await;

        let base = dep
            .base_url()
            .unwrap_or_else(|| panic!("{}: base_url after start", case.name))
            .trim_end_matches('/')
            .to_string();
        let client = reqwest::Client::new();

        let jwks_url = format!("{base}{}", case.expected_jwks_path);
        let jwks: Value = client
            .get(&jwks_url)
            .send()
            .await
            .unwrap_or_else(|e| panic!("{}: jwks GET failed: {e}", case.name))
            .error_for_status()
            .unwrap_or_else(|e| panic!("{}: jwks status: {e}", case.name))
            .json()
            .await
            .unwrap_or_else(|e| panic!("{}: jwks JSON: {e}", case.name));
        assert!(
            jwks["keys"][0]["kid"].as_str().is_some(),
            "{}: expected kid in jwks",
            case.name
        );

        let issuer = dep
            .issuer_at(0)
            .unwrap_or_else(|| panic!("{}: issuer_at", case.name));
        assert_eq!(
            issuer,
            format!("{base}{}", case.expected_issuer_path),
            "{}: issuer path mismatch",
            case.name
        );

        let claims = json!({ "iss": issuer, "sub": "test-subject", "exp": 9_999_999_999u64, "iat": 0 });
        let token = dep
            .sign_claims(0, &claims)
            .unwrap_or_else(|e| panic!("{}: sign_claims: {e}", case.name));
        dep.verify_access_token(&token)
            .unwrap_or_else(|e| panic!("{}: verify_access_token: {e:?}", case.name));

        dep.stop().await;
    }
}
