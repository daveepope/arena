use std::net::{IpAddr, Ipv4Addr};

use arena::Dependency;
use serde::Deserialize;

use crate::OauthDependency;

#[derive(Debug, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OauthFfiInboundTransport {
    #[default]
    Tls,
    Http,
}

#[derive(Debug, Deserialize)]
pub struct OauthFfiDependencyConfig {
    pub identifier: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub listen_ip: Option<String>,
    #[serde(default)]
    pub server_tls_certificate_pem: Option<String>,
    #[serde(default)]
    pub server_tls_private_key_pem: Option<String>,
    #[serde(default)]
    pub metadata_base_url: Option<String>,
    #[serde(default)]
    pub transport: Option<OauthFfiInboundTransport>,
}

pub fn build_oauth_dependency_from_config(
    config: &OauthFfiDependencyConfig,
    _network: Option<&str>,
) -> Result<Dependency, String> {
    let listen_ip = match config.listen_ip.as_deref() {
        Some(s) if !s.trim().is_empty() => s
            .parse::<IpAddr>()
            .map_err(|e| format!("oauth listen_ip: invalid IP: {e}"))?,
        _ => IpAddr::V4(Ipv4Addr::LOCALHOST),
    };
    let port = config.port.unwrap_or(0);
    let mut builder = OauthDependency::builder(&config.identifier)
        .with_listen_ip(listen_ip)
        .with_port(port);
    match config.transport.unwrap_or_default() {
        OauthFfiInboundTransport::Http => {
            builder = builder.with_http();
        }
        OauthFfiInboundTransport::Tls => {
            if let (Some(cert), Some(key)) = (
                config.server_tls_certificate_pem.as_deref(),
                config.server_tls_private_key_pem.as_deref(),
            ) {
                if !cert.trim().is_empty() && !key.trim().is_empty() {
                    builder = builder.with_server_tls_pem(cert.to_string(), key.to_string());
                } else {
                    builder = builder.with_ephemeral_server_tls();
                }
            } else {
                builder = builder.with_ephemeral_server_tls();
            }
        }
    }
    if let Some(ref u) = config.metadata_base_url {
        let t = u.trim();
        if !t.is_empty() {
            builder = builder.with_metadata_base_url(t.to_string());
        }
    }
    Ok(Box::new(builder.build()))
}
