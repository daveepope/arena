use arena_oauth::{
    build_oauth_dependency_from_config, OauthFfiDependencyConfig, OauthFfiInboundTransport,
};

fn base_config(identifier: &str) -> OauthFfiDependencyConfig {
    OauthFfiDependencyConfig {
        identifier: identifier.to_string(),
        port: None,
        listen_ip: None,
        server_tls_certificate_pem: None,
        server_tls_private_key_pem: None,
        metadata_base_url: None,
        transport: None,
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
