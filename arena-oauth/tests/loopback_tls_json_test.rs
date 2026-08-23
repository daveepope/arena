use arena_oauth::loopback_tls_pem_json_document;
use serde_json::Value;

#[test]
fn loopback_tls_pem_json_document_contains_cert_and_key() {
    let json = loopback_tls_pem_json_document().expect("generate loopback tls pem json");
    let parsed: Value = serde_json::from_str(&json).expect("parse json document");

    let cert = parsed["certificate_pem"]
        .as_str()
        .expect("certificate_pem string");
    let key = parsed["private_key_pem"].as_str().expect("private_key_pem string");

    assert!(!cert.is_empty());
    assert!(!key.is_empty());
    assert_ne!(cert, key);
}
