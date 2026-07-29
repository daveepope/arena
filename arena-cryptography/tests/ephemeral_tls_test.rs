use arena_cryptography::ephemeral_tls::localhost_self_signed_pem_pair;

#[test]
fn localhost_self_signed_pem_pair_returns_valid_pems() {
    let (cert_pem, key_pem) = localhost_self_signed_pem_pair().unwrap();

    assert!(cert_pem.contains("-----BEGIN CERTIFICATE-----"));
    assert!(cert_pem.contains("-----END CERTIFICATE-----"));
    assert!(key_pem.contains("-----BEGIN PRIVATE KEY-----"));
    assert!(key_pem.contains("-----END PRIVATE KEY-----"));
}

#[test]
fn localhost_self_signed_pem_pair_is_fresh_per_call() {
    let (_, first_key) = localhost_self_signed_pem_pair().unwrap();
    let (_, second_key) = localhost_self_signed_pem_pair().unwrap();

    assert_ne!(first_key, second_key);
}
