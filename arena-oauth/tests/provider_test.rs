use arena_oauth::OauthDependency;

fn builder_with_provider(name: &str, provider: arena_oauth::Provider) -> OauthDependency {
    OauthDependency::builder(name)
        .with_http()
        .with_provider(provider)
        .build()
}

#[test]
fn issuer_path_cognito_pool_id_returns_pool_prefixed_path() {
    let dep = builder_with_provider(
        "provider-cognito-issuer-path",
        arena_oauth::Provider::Cognito {
            pool_id: "us-east-1_abc123".to_string(),
        },
    );
    assert_eq!(dep.issuer_path_at(0), Some("/us-east-1_abc123"));
}

#[test]
fn jwks_path_cognito_pool_id_returns_pool_prefixed_jwks_path() {
    let dep = builder_with_provider(
        "provider-cognito-jwks-path",
        arena_oauth::Provider::Cognito {
            pool_id: "us-east-1_abc123".to_string(),
        },
    );
    assert_eq!(
        dep.jwks_path_at(0),
        Some("/us-east-1_abc123/.well-known/jwks.json")
    );
}

#[test]
fn issuer_path_okta_returns_empty_path() {
    let dep = builder_with_provider("provider-okta-issuer-path", arena_oauth::Provider::Okta);
    assert_eq!(dep.issuer_path_at(0), Some(""));
}

#[test]
fn jwks_path_okta_returns_v1_keys() {
    let dep = builder_with_provider("provider-okta-jwks-path", arena_oauth::Provider::Okta);
    assert_eq!(dep.jwks_path_at(0), Some("/v1/keys"));
}

#[test]
fn issuer_path_entra_id_tenant_id_returns_v2_path() {
    let dep = builder_with_provider(
        "provider-entra-id-issuer-path",
        arena_oauth::Provider::EntraId {
            tenant_id: "my-tenant".to_string(),
        },
    );
    assert_eq!(dep.issuer_path_at(0), Some("/my-tenant/v2.0"));
}

#[test]
fn jwks_path_entra_id_tenant_id_returns_discovery_v2_keys_path() {
    let dep = builder_with_provider(
        "provider-entra-id-jwks-path",
        arena_oauth::Provider::EntraId {
            tenant_id: "my-tenant".to_string(),
        },
    );
    assert_eq!(
        dep.jwks_path_at(0),
        Some("/my-tenant/discovery/v2.0/keys")
    );
}
