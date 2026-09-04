use std::net::{IpAddr, Ipv4Addr};
use std::sync::OnceLock;

use arena_oauth::{IssuerConfig, OauthDependency, Provider};
use rsa::pkcs8::{EncodePrivateKey, LineEnding};
use rsa::RsaPrivateKey;

static PKCS8_PEM: OnceLock<String> = OnceLock::new();

fn generate_pkcs8_pem() -> String {
    PKCS8_PEM
        .get_or_init(|| {
            let mut rng = rand::thread_rng();
            let key = RsaPrivateKey::new(&mut rng, 2048).expect("generate rsa key");
            key.to_pkcs8_pem(LineEnding::LF)
                .expect("encode pkcs8 pem")
                .to_string()
        })
        .clone()
}

#[test]
fn build_with_ephemeral_server_tls_exposes_certificate() {
    let dep = OauthDependency::builder("oauth-builder-ephemeral")
        .with_ephemeral_server_tls()
        .build();
    assert!(dep.server_tls_certificate_pem().is_some());
}

#[test]
fn build_with_port_and_listen_ip_does_not_panic() {
    let dep = OauthDependency::builder("oauth-builder-port")
        .with_http()
        .with_port(0)
        .with_listen_ip(IpAddr::V4(Ipv4Addr::LOCALHOST))
        .build();
    assert!(dep.server_tls_certificate_pem().is_none());
}

#[test]
fn build_with_valid_rsa_pkcs8_pem_succeeds() {
    let pem = generate_pkcs8_pem();
    let dep = OauthDependency::builder("oauth-builder-rsa")
        .with_http()
        .with_rsa_pkcs8_pem(pem)
        .build();
    assert!(dep.server_tls_certificate_pem().is_none());
}

#[test]
#[should_panic(expected = "invalid PKCS#8 PEM")]
fn build_with_invalid_rsa_pkcs8_pem_panics() {
    OauthDependency::builder("oauth-builder-bad-rsa")
        .with_rsa_pkcs8_pem("not a pem")
        .build();
}

#[test]
fn build_with_scopes_token_ttl_and_metadata_base_url_does_not_panic() {
    let dep = OauthDependency::builder("oauth-builder-options")
        .with_http()
        .with_scopes_supported(vec!["read".into(), "write".into()])
        .with_token_ttl_secs(120)
        .with_metadata_base_url("https://issuer.example")
        .build();
    assert!(dep.server_tls_certificate_pem().is_none());
}

#[test]
#[should_panic(expected = "must be non-empty")]
fn build_with_empty_server_tls_pem_panics() {
    OauthDependency::builder("oauth-builder-empty-pem")
        .with_server_tls_pem("", "")
        .build();
}

#[test]
fn build_with_no_issuers_defaults_to_root_jwks_path() {
    let dep = OauthDependency::builder("oauth-builder-default-issuer")
        .with_http()
        .build();
    assert_eq!(dep.issuer_count(), 1);
    assert_eq!(dep.issuer_path_at(0), Some(""));
    assert_eq!(dep.jwks_path_at(0), Some("/.well-known/jwks.json"));
}

#[test]
fn build_with_issuer_and_provider_registers_distinct_jwks_routes() {
    let dep = OauthDependency::builder("oauth-builder-issuer-and-provider")
        .with_http()
        .with_provider(Provider::Cognito {
            pool_id: "pool-a".to_string(),
        })
        .with_issuer(IssuerConfig::new().with_jwks_path("/custom/keys"))
        .build();
    assert_eq!(dep.issuer_count(), 2);
    assert_eq!(dep.jwks_path_at(0), Some("/pool-a/.well-known/jwks.json"));
    assert_eq!(dep.jwks_path_at(1), Some("/custom/keys"));
}

#[test]
fn build_with_provider_cognito_resolves_pool_id_path() {
    let dep = OauthDependency::builder("oauth-builder-provider-cognito")
        .with_http()
        .with_provider(Provider::Cognito {
            pool_id: "us-east-1_abc123".to_string(),
        })
        .build();
    assert_eq!(dep.issuer_path_at(0), Some("/us-east-1_abc123"));
    assert_eq!(
        dep.jwks_path_at(0),
        Some("/us-east-1_abc123/.well-known/jwks.json")
    );
}

#[test]
fn build_with_provider_entra_id_resolves_tenant_path() {
    let dep = OauthDependency::builder("oauth-builder-provider-entra-id")
        .with_http()
        .with_provider(Provider::EntraId {
            tenant_id: "my-tenant".to_string(),
        })
        .build();
    assert_eq!(dep.issuer_path_at(0), Some("/my-tenant/v2.0"));
    assert_eq!(
        dep.jwks_path_at(0),
        Some("/my-tenant/discovery/v2.0/keys")
    );
}

#[test]
#[should_panic(expected = "duplicate JWKS path")]
fn build_with_duplicate_jwks_path_panics() {
    OauthDependency::builder("oauth-builder-duplicate-jwks-path")
        .with_http()
        .with_provider(Provider::Okta)
        .with_issuer(
            IssuerConfig::new()
                .with_issuer_path("/other")
                .with_jwks_path("/v1/keys"),
        )
        .build();
}

#[test]
#[should_panic(expected = "mutually exclusive")]
fn build_with_global_pem_and_issuer_panics() {
    let pem = generate_pkcs8_pem();
    OauthDependency::builder("oauth-builder-global-pem-and-issuer")
        .with_http()
        .with_rsa_pkcs8_pem(pem)
        .with_provider(Provider::Okta)
        .build();
}

#[test]
#[should_panic(expected = "duplicate issuer path")]
fn build_with_duplicate_issuer_path_panics() {
    OauthDependency::builder("oauth-builder-duplicate-issuer-path")
        .with_http()
        .with_provider(Provider::Okta)
        .with_issuer(IssuerConfig::new().with_jwks_path("/other/keys"))
        .build();
}

#[test]
#[should_panic(expected = "duplicate JWKS path")]
fn build_with_jwks_path_colliding_with_reserved_route_panics() {
    OauthDependency::builder("oauth-builder-reserved-route-collision")
        .with_http()
        .with_issuer(
            IssuerConfig::new()
                .with_issuer_path("/custom")
                .with_jwks_path("/oauth/token"),
        )
        .build();
}

#[test]
fn build_with_issuer_path_and_no_jwks_path_scopes_default_jwks_path_to_issuer() {
    let dep = OauthDependency::builder("oauth-builder-scoped-default-jwks")
        .with_http()
        .with_issuer(IssuerConfig::new().with_issuer_path("/tenant-a"))
        .with_issuer(IssuerConfig::new().with_issuer_path("/tenant-b"))
        .build();
    assert_eq!(dep.issuer_count(), 2);
    assert_eq!(
        dep.jwks_path_at(0),
        Some("/tenant-a/.well-known/jwks.json")
    );
    assert_eq!(
        dep.jwks_path_at(1),
        Some("/tenant-b/.well-known/jwks.json")
    );
}
