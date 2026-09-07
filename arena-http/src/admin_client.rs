pub(crate) fn admin_api_client(
    admin_url: &str,
    trusted_certificate_pem: Option<&str>,
) -> Result<reqwest::Client, String> {
    let mut b = reqwest::Client::builder();

    if let Some(pem) = trusted_certificate_pem
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        let cert = reqwest::Certificate::from_pem(pem.as_bytes()).map_err(|e| {
            format!("invalid trusted TLS certificate PEM for HTTP admin client: {e}")
        })?;
        b = b.add_root_certificate(cert);
    } else if admin_url.starts_with("https://") {
        if !is_loopback_url(admin_url) {
            return Err(format!(
                "HTTP admin API at {admin_url} is served over TLS by a host Arena cannot verify. \
                 Pass the server certificate with with_trusted_certificate_pem(...)"
            ));
        }
        b = b.danger_accept_invalid_certs(true);
    }

    b.build()
        .map_err(|e| format!("reqwest client for HTTP admin API: {e}"))
}

fn is_loopback_url(url: &str) -> bool {
    let without_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let authority = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    let host = match authority.rsplit_once(']') {
        Some((bracketed, _)) => bracketed.trim_start_matches('['),
        None => authority.split(':').next().unwrap_or_default(),
    };

    match host.parse::<std::net::IpAddr>() {
        Ok(ip) => ip.is_loopback(),
        Err(_) => host.eq_ignore_ascii_case("localhost"),
    }
}
