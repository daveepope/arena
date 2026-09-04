use arena_cryptography::ephemeral_tls::{localhost_self_signed_pem_pair, self_signed_pem_pair};

#[test]
fn localhost_self_signed_pem_pair_returns_valid_pems() {
    let (cert_pem, key_pem) = localhost_self_signed_pem_pair().unwrap();

    assert!(!cert_pem.is_empty());
    assert!(!key_pem.is_empty());
    assert_ne!(cert_pem, key_pem);
}

#[test]
fn localhost_self_signed_pem_pair_is_fresh_per_call() {
    let (_, first_key) = localhost_self_signed_pem_pair().unwrap();
    let (_, second_key) = localhost_self_signed_pem_pair().unwrap();

    assert_ne!(first_key, second_key);
}

#[test]
fn self_signed_pem_pair_ipv6_and_dns_hosts_returns_valid_pems() {
    let hosts = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];

    let (cert_pem, key_pem) = self_signed_pem_pair(&hosts).unwrap();

    assert!(cert_pem.starts_with("-----BEGIN CERTIFICATE-----"));
    assert!(key_pem.contains("PRIVATE KEY"));
}
