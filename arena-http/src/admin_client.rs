pub(crate) fn admin_api_client(
    admin_url: &str,
    trusted_certificate_pem: Option<&str>,
) -> reqwest::Client {
    let mut b = reqwest::Client::builder();

    if let Some(pem) = trusted_certificate_pem
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        let cert = reqwest::Certificate::from_pem(pem.as_bytes()).unwrap_or_else(|e| {
            panic!("invalid trusted TLS certificate PEM for HTTP admin client: {e}")
        });
        b = b.add_root_certificate(cert);
    } else if admin_url.starts_with("https://") {
        b = b.danger_accept_invalid_certs(true);
    }

    b.build()
        .unwrap_or_else(|e| panic!("reqwest client for HTTP admin API: {e}"))
}
