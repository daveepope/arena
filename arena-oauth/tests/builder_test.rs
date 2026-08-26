use std::net::{IpAddr, Ipv4Addr};

use arena_oauth::OauthDependency;
use rsa::pkcs8::{EncodePrivateKey, LineEnding};
use rsa::RsaPrivateKey;

fn generate_pkcs8_pem() -> String {
    let mut rng = rand::thread_rng();
    let key = RsaPrivateKey::new(&mut rng, 2048).expect("generate rsa key");
    key.to_pkcs8_pem(LineEnding::LF)
        .expect("encode pkcs8 pem")
        .to_string()
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
