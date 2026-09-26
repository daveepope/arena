use rcgen::{
    date_time_ymd, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, KeyPair,
    KeyUsagePurpose,
};

pub fn localhost_self_signed_pem_pair() -> Result<(String, String), String> {
    self_signed_pem_pair(&["localhost".to_string(), "127.0.0.1".to_string()])
}

pub fn self_signed_pem_pair(subject_alt_names: &[String]) -> Result<(String, String), String> {
    let key_pair = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e: rcgen::Error| e.to_string())?;

    let mut params = CertificateParams::new(subject_alt_names.to_vec())
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
