use std::net::{IpAddr, Ipv4Addr};

use arena::dependency::RunnableDependency;

use crate::keys::RsaKeyPair;
use crate::oauth_common::OauthListenAddr;
use crate::oauth_dependency::{OauthDependency, OauthTlsPlan};

enum InboundTransport {
    EphemeralTls,
    CustomTls { cert_pem: String, key_pem: String },
}

pub struct OauthDependencyBuilder {
    identifier: String,
    listen_ip: Option<IpAddr>,
    port: Option<u16>,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    rsa_pkcs8_pem: Option<String>,
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
            scopes_supported: vec!["openid".into(), "profile".into()],
            token_ttl_secs: None,
            inbound_transport: InboundTransport::EphemeralTls,
            metadata_base_url: None,
        }
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
        let keys = match self.rsa_pkcs8_pem {
            Some(pem) => RsaKeyPair::from_pkcs8_pem(&pem, RsaKeyPair::DEFAULT_KID)
                .unwrap_or_else(|e| panic!("[Oauth-{}] invalid PKCS#8 PEM: {e}", self.identifier)),
            None => RsaKeyPair::generate()
                .unwrap_or_else(|e| panic!("[Oauth-{}] RSA generate failed: {e}", self.identifier)),
        };
        let port = self.port.unwrap_or(0);
        let listen_ip = self
            .listen_ip
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        let token_ttl_secs = self.token_ttl_secs.unwrap_or(Self::DEFAULT_TOKEN_TTL_SECS);
        let tls_plan = match self.inbound_transport {
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
            keys,
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
