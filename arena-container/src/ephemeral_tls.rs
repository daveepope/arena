use rcgen::{
    date_time_ymd, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, KeyPair,
    KeyUsagePurpose,
};

pub fn localhost_self_signed_pem_pair() -> Result<(String, String), String> {
    let key_pair = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e: rcgen::Error| e.to_string())?;

    let mut params =
        CertificateParams::new(vec!["localhost".to_string(), "127.0.0.1".to_string()])
            .map_err(|e: rcgen::Error| e.to_string())?;

    params.not_before = date_time_ymd(2024, 1, 1);
    params.not_after = date_time_ymd(2099, 12, 31);
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "arena-ephemeral");
    params.distinguished_name = dn;

    let cert = params
        .self_signed(&key_pair)
        .map_err(|e: rcgen::Error| e.to_string())?;
    Ok((cert.pem(), key_pair.serialize_pem()))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
