use arena::Dependency;
use arena_oauth::{
    build_oauth_dependency_from_config, OauthDependency, OauthFfiDependencyConfig,
    OauthFfiInboundTransport, OauthFfiIssuerConfig,
};

fn as_oauth(dep: &Dependency) -> &OauthDependency {
    dep.as_any()
        .downcast_ref::<OauthDependency>()
        .expect("dependency is an OauthDependency")
}

fn base_config(identifier: &str) -> OauthFfiDependencyConfig {
    OauthFfiDependencyConfig {
        identifier: identifier.to_string(),
        port: None,
        listen_ip: None,
        server_tls_certificate_pem: None,
        server_tls_private_key_pem: None,
        metadata_base_url: None,
        transport: None,
        issuers: Vec::new(),
    }
}

#[test]
fn build_from_config_default_transport_uses_ephemeral_tls() {
    let config = base_config("ffi-default");
    let dep = build_oauth_dependency_from_config(&config, None).expect("build dependency");
    assert!(dep.identifier().contains("ffi-default"));
}

#[test]
fn build_from_config_http_transport_selected() {
    let mut config = base_config("ffi-http");
    config.transport = Some(OauthFfiInboundTransport::Http);
    let dep = build_oauth_dependency_from_config(&config, None).expect("build dependency");
    assert!(dep.identifier().contains("ffi-http"));
}

#[test]
fn build_from_config_tls_transport_with_blank_pem_falls_back_to_ephemeral() {
    let mut config = base_config("ffi-blank-pem");
    config.transport = Some(OauthFfiInboundTransport::Tls);
    config.server_tls_certificate_pem = Some("  ".to_string());
    config.server_tls_private_key_pem = Some("  ".to_string());
    let dep = build_oauth_dependency_from_config(&config, None).expect("build dependency");
    assert!(dep.identifier().contains("ffi-blank-pem"));
}

#[test]
fn build_from_config_tls_transport_missing_pem_falls_back_to_ephemeral() {
    let mut config = base_config("ffi-missing-pem");
    config.transport = Some(OauthFfiInboundTransport::Tls);
    let dep = build_oauth_dependency_from_config(&config, None).expect("build dependency");
    assert!(dep.identifier().contains("ffi-missing-pem"));
}

#[test]
fn build_from_config_invalid_listen_ip_returns_err() {
    let mut config = base_config("ffi-bad-ip");
    config.listen_ip = Some("not-an-ip".to_string());
    let err = match build_oauth_dependency_from_config(&config, None) {
        Err(e) => e,
        Ok(_) => panic!("expected invalid IP error"),
    };
    assert!(err.contains("invalid IP"));
}

#[test]
fn build_from_config_blank_listen_ip_uses_loopback_default() {
    let mut config = base_config("ffi-blank-ip");
    config.listen_ip = Some("   ".to_string());
    let dep = build_oauth_dependency_from_config(&config, None).expect("build dependency");
    assert!(dep.identifier().contains("ffi-blank-ip"));
}

#[test]
fn build_from_config_blank_metadata_base_url_is_ignored() {
    let mut config = base_config("ffi-blank-metadata");
    config.metadata_base_url = Some("   ".to_string());
    let dep = build_oauth_dependency_from_config(&config, None).expect("build dependency");
    assert!(dep.identifier().contains("ffi-blank-metadata"));
}

#[test]
fn build_from_config_explicit_port_and_metadata_base_url_applied() {
    let mut config = base_config("ffi-explicit");
    config.port = Some(0);
    config.metadata_base_url = Some("https://issuer.example".to_string());
    let dep = build_oauth_dependency_from_config(&config, None).expect("build dependency");
    assert!(dep.identifier().contains("ffi-explicit"));
}

#[test]
fn inbound_transport_default_is_tls() {
    assert_eq!(
        OauthFfiInboundTransport::default(),
        OauthFfiInboundTransport::Tls
    );
}

#[test]
fn build_from_config_with_cognito_issuer_resolves_pool_id_path() {
    let mut config = base_config("ffi-cognito");
    config.issuers = vec![OauthFfiIssuerConfig::Cognito {
        pool_id: "us-east-1_abc123".to_string(),
    }];
    let dep = build_oauth_dependency_from_config(&config, None).expect("build dependency");
    let dep = as_oauth(&dep);
    assert_eq!(
        dep.jwks_path_at(0),
        Some("/us-east-1_abc123/.well-known/jwks.json")
    );
    assert_eq!(dep.issuer_path_at(0), Some("/us-east-1_abc123"));
}

#[test]
fn build_from_config_with_okta_issuer_resolves_v1_keys_path() {
    let mut config = base_config("ffi-okta");
    config.issuers = vec![OauthFfiIssuerConfig::Okta];
    let dep = build_oauth_dependency_from_config(&config, None).expect("build dependency");
    let dep = as_oauth(&dep);
    assert_eq!(dep.jwks_path_at(0), Some("/v1/keys"));
    assert_eq!(dep.issuer_path_at(0), Some(""));
}

#[test]
fn build_from_config_with_entra_id_issuer_resolves_tenant_path() {
    let mut config = base_config("ffi-entra-id");
    config.issuers = vec![OauthFfiIssuerConfig::EntraId {
        tenant_id: "my-tenant".to_string(),
    }];
    let dep = build_oauth_dependency_from_config(&config, None).expect("build dependency");
    let dep = as_oauth(&dep);
    assert_eq!(dep.jwks_path_at(0), Some("/my-tenant/discovery/v2.0/keys"));
    assert_eq!(dep.issuer_path_at(0), Some("/my-tenant/v2.0"));
}

#[test]
fn build_from_config_with_custom_issuer_uses_literal_paths() {
    let mut config = base_config("ffi-custom");
    config.issuers = vec![OauthFfiIssuerConfig::Custom {
        issuer_path: Some("/custom".to_string()),
        jwks_path: Some("/custom/keys".to_string()),
        rsa_pkcs8_pem: None,
    }];
    let dep = build_oauth_dependency_from_config(&config, None).expect("build dependency");
    let dep = as_oauth(&dep);
    assert_eq!(dep.jwks_path_at(0), Some("/custom/keys"));
    assert_eq!(dep.issuer_path_at(0), Some("/custom"));
}

#[test]
fn build_from_config_with_multiple_issuers_builds_all_registrations() {
    let mut config = base_config("ffi-multi");
    config.issuers = vec![
        OauthFfiIssuerConfig::Cognito {
            pool_id: "pool-a".to_string(),
        },
        OauthFfiIssuerConfig::Okta,
    ];
    let dep = build_oauth_dependency_from_config(&config, None).expect("build dependency");
    let dep = as_oauth(&dep);
    assert_eq!(dep.issuer_count(), 2);
    assert_eq!(dep.jwks_path_at(0), Some("/pool-a/.well-known/jwks.json"));
    assert_eq!(dep.jwks_path_at(1), Some("/v1/keys"));
}
