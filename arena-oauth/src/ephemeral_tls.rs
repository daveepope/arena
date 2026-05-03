use rcgen::{CertificateParams, KeyPair};

pub(crate) fn localhost_self_signed_certificate_and_key() -> Result<(String, String), String> {
    let key_pair = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e: rcgen::Error| e.to_string())?;
    let params = CertificateParams::new(vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
    ])
    .map_err(|e: rcgen::Error| e.to_string())?;
    let cert = params
        .self_signed(&key_pair)
        .map_err(|e: rcgen::Error| e.to_string())?;
    Ok((cert.pem(), key_pair.serialize_pem()))
}
