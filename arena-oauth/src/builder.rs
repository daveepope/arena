use std::net::{IpAddr, Ipv4Addr};

use arena::dependency::RunnableDependency;

use crate::keys::RsaKeyPair;
use crate::oauth_common::{IssuerRegistration, OauthListenAddr};
use crate::oauth_dependency::{OauthDependency, OauthTlsPlan};
use crate::provider::Provider;

pub(crate) const DEFAULT_JWKS_PATH: &str = "/.well-known/jwks.json";

enum InboundTransport {
    Http,
    EphemeralTls,
    CustomTls { cert_pem: String, key_pem: String },
}

#[derive(Default)]
pub struct IssuerConfig {
    issuer_path: Option<String>,
    jwks_path: Option<String>,
    rsa_pkcs8_pem: Option<String>,
    provider: Option<Provider>,
}

impl IssuerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_issuer_path(mut self, path: impl Into<String>) -> Self {
        self.issuer_path = Some(path.into());
        self
    }

    pub fn with_jwks_path(mut self, path: impl Into<String>) -> Self {
        self.jwks_path = Some(path.into());
        self
    }

    pub fn with_rsa_pkcs8_pem(mut self, pem: impl Into<String>) -> Self {
        self.rsa_pkcs8_pem = Some(pem.into());
        self
    }
}

pub struct OauthDependencyBuilder {
    identifier: String,
    listen_ip: Option<IpAddr>,
    port: Option<u16>,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    rsa_pkcs8_pem: Option<String>,
    issuers: Vec<IssuerConfig>,
    scopes_supported: Vec<String>,
    token_ttl_secs: Option<u64>,
    inbound_transport: InboundTransport,
    metadata_base_url: Option<String>,
}

impl OauthDependencyBuilder {
    const DEFAULT_TOKEN_TTL_SECS: u64 = 3600;

    pub(crate) fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            listen_ip: None,
            port: None,
            dependencies: None,
            rsa_pkcs8_pem: None,
            issuers: Vec::new(),
            scopes_supported: vec!["openid".into(), "profile".into()],
            token_ttl_secs: None,
            inbound_transport: InboundTransport::EphemeralTls,
            metadata_base_url: None,
        }
    }

    pub fn with_http(mut self) -> Self {
        self.inbound_transport = InboundTransport::Http;
        self
    }

    /// Ephemeral self-signed TLS for `localhost` / `127.0.0.1` (default).
    pub fn with_ephemeral_server_tls(mut self) -> Self {
        self.inbound_transport = InboundTransport::EphemeralTls;
        self
    }

    pub fn with_server_tls_pem(
        mut self,
        certificate_pem: impl Into<String>,
        private_key_pem: impl Into<String>,
    ) -> Self {
        self.inbound_transport = InboundTransport::CustomTls {
            cert_pem: certificate_pem.into(),
            key_pem: private_key_pem.into(),
        };
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn with_listen_ip(mut self, ip: IpAddr) -> Self {
        self.listen_ip = Some(ip);
        self
    }

    pub fn with_rsa_pkcs8_pem(mut self, pem: impl Into<String>) -> Self {
        self.rsa_pkcs8_pem = Some(pem.into());
        self
    }

    pub fn with_issuer(mut self, config: IssuerConfig) -> Self {
        self.issuers.push(config);
        self
    }

    pub fn with_provider(self, provider: Provider) -> Self {
        let config = IssuerConfig {
            issuer_path: Some(provider.issuer_path()),
            jwks_path: Some(provider.jwks_path()),
            rsa_pkcs8_pem: None,
            provider: Some(provider),
        };
        self.with_issuer(config)
    }

    pub fn with_scopes_supported(mut self, scopes: Vec<String>) -> Self {
        self.scopes_supported = scopes;
        self
    }

    pub fn with_token_ttl_secs(mut self, secs: u64) -> Self {
        self.token_ttl_secs = Some(secs);
        self
    }

    pub fn with_metadata_base_url(mut self, url: impl Into<String>) -> Self {
        self.metadata_base_url = Some(url.into());
        self
    }

    pub fn with_child_dependencies(
        mut self,
        dependencies: Vec<Box<dyn RunnableDependency>>,
    ) -> Self {
        self.dependencies = Some(dependencies);
        self
    }

    pub fn build(self) -> OauthDependency {
        if !self.issuers.is_empty() && self.rsa_pkcs8_pem.is_some() {
            panic!(
                "[Oauth-{}] with_rsa_pkcs8_pem and with_issuer/with_provider are mutually exclusive; set the key per issuer",
                self.identifier
            );
        }

        let issuers: Vec<IssuerRegistration> = if self.issuers.is_empty() {
            let keys = match &self.rsa_pkcs8_pem {
                Some(pem) => RsaKeyPair::from_pkcs8_pem(pem, RsaKeyPair::DEFAULT_KID)
                    .unwrap_or_else(|e| {
                        panic!("[Oauth-{}] invalid PKCS#8 PEM: {e}", self.identifier)
                    }),
                None => RsaKeyPair::generate().unwrap_or_else(|e| {
                    panic!("[Oauth-{}] RSA generate failed: {e}", self.identifier)
                }),
            };
            vec![IssuerRegistration {
                provider: Provider::Custom {
                    issuer_path: Some(String::new()),
                },
                issuer_path: String::new(),
                jwks_path: DEFAULT_JWKS_PATH.to_string(),
                keys,
            }]
        } else {
            let mut seen_jwks_paths: std::collections::HashSet<String> =
                crate::oauth_https::RESERVED_PATHS
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            let mut seen_issuer_paths = std::collections::HashSet::new();
            self.issuers
                .into_iter()
                .enumerate()
                .map(|(i, config)| {
                    let issuer_path = config.issuer_path.unwrap_or_default();
                    let provider = config.provider.clone().unwrap_or_else(|| Provider::Custom {
                        issuer_path: Some(issuer_path.clone()),
                    });
                    if !seen_issuer_paths.insert(issuer_path.clone()) {
                        panic!(
                            "[Oauth-{}] duplicate issuer path registered: {issuer_path:?}",
                            self.identifier
                        );
                    }
                    let jwks_path = config.jwks_path.unwrap_or_else(|| {
                        if issuer_path.is_empty() {
                            DEFAULT_JWKS_PATH.to_string()
                        } else {
                            format!("{issuer_path}{DEFAULT_JWKS_PATH}")
                        }
                    });
                    if !seen_jwks_paths.insert(jwks_path.clone()) {
                        panic!(
                            "[Oauth-{}] duplicate JWKS path registered: {jwks_path}",
                            self.identifier
                        );
                    }
                    let kid = format!("arena-oauth-{}", i + 1);
                    let keys = match &config.rsa_pkcs8_pem {
                        Some(pem) => RsaKeyPair::from_pkcs8_pem(pem, kid).unwrap_or_else(|e| {
                            panic!("[Oauth-{}] invalid PKCS#8 PEM: {e}", self.identifier)
                        }),
                        None => RsaKeyPair::generate_with_kid(kid).unwrap_or_else(|e| {
                            panic!("[Oauth-{}] RSA generate failed: {e}", self.identifier)
                        }),
                    };
                    IssuerRegistration {
                        provider,
                        issuer_path,
                        jwks_path,
                        keys,
                    }
                })
                .collect()
        };

        let port = self.port.unwrap_or(0);
        let listen_ip = self.listen_ip.unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        let token_ttl_secs = self.token_ttl_secs.unwrap_or(Self::DEFAULT_TOKEN_TTL_SECS);
        let tls_plan = match self.inbound_transport {
            InboundTransport::Http => OauthTlsPlan::Disabled,
            InboundTransport::EphemeralTls => OauthTlsPlan::EphemeralOnStart,
            InboundTransport::CustomTls { cert_pem, key_pem } => {
                if cert_pem.trim().is_empty() || key_pem.trim().is_empty() {
                    panic!(
                        "[Oauth-{}] TLS certificate PEM and private key PEM must be non-empty",
                        self.identifier
                    );
                }
                OauthTlsPlan::CustomPem { cert_pem, key_pem }
            }
        };
        OauthDependency::new(
            arena_container::identifier::build("arena-oauth", &self.identifier),
            issuers,
            OauthListenAddr {
                ip: listen_ip,
                port,
            },
            self.dependencies,
            self.scopes_supported,
            token_ttl_secs,
            tls_plan,
            self.metadata_base_url,
        )
    }
}
