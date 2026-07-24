use std::time::Instant;

use arena::dependency::RunnableDependency;
use async_trait::async_trait;

use crate::builder::OauthDependencyBuilder;
use arena_container::ephemeral_tls;
use crate::keys::RsaKeyPair;
use crate::oauth_common::OauthListenAddr;
use crate::oauth_server::OauthServer;
use crate::token::{AccessTokenClaims, TokenError};

pub(crate) enum OauthTlsPlan {
    Disabled,
    EphemeralOnStart,
    CustomPem { cert_pem: String, key_pem: String },
}

pub struct OauthDependency {
    pub identifier: String,
    listen: OauthListenAddr,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    running: bool,
    needs_teardown: bool,
    children_started: bool,
    keys: RsaKeyPair,
    scopes_supported: Vec<String>,
    token_ttl_secs: u64,
    active_server_tls: Option<(String, String)>,
    metadata_base_url: Option<String>,
    oauth_server: OauthServer,
}

impl OauthDependency {
    pub(crate) fn new(
        identifier: String,
        keys: RsaKeyPair,
        listen: OauthListenAddr,
        dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
        scopes_supported: Vec<String>,
        token_ttl_secs: u64,
        tls_plan: OauthTlsPlan,
        metadata_base_url: Option<String>,
    ) -> Self {
        let active_server_tls = match &tls_plan {
            OauthTlsPlan::Disabled => None,
            OauthTlsPlan::CustomPem { cert_pem, key_pem } => {
                Some((cert_pem.clone(), key_pem.clone()))
            }
            OauthTlsPlan::EphemeralOnStart => Some(
                ephemeral_tls::localhost_self_signed_pem_pair().unwrap_or_else(|e| {
                    panic!(
                        "[Oauth-{}] ephemeral TLS certificate generation failed: {e}",
                        identifier
                    )
                }),
            ),
        };
        Self {
            identifier,
            listen,
            dependencies,
            running: false,
            needs_teardown: false,
            children_started: false,
            keys,
            scopes_supported,
            token_ttl_secs,
            active_server_tls,
            metadata_base_url,
            oauth_server: OauthServer::default(),
        }
    }

    pub fn builder(identifier: impl Into<String>) -> OauthDependencyBuilder {
        OauthDependencyBuilder::new(identifier)
    }

    pub fn base_url(&self) -> Option<&str> {
        self.oauth_server.base_url()
    }

    pub fn issuer(&self) -> Option<String> {
        self.base_url().map(|b| b.trim_end_matches('/').to_string())
    }

    pub fn server_tls_certificate_pem(&self) -> Option<&str> {
        self.active_server_tls.as_ref().map(|(c, _)| c.as_str())
    }

    pub fn verify_access_token(&self, token: &str) -> Result<AccessTokenClaims, TokenError> {
        let state = self
            .oauth_server
            .signing_state()
            .ok_or(TokenError::NotRunning)?;
        crate::token::verify_access_token(token, state.keys.as_ref(), &state.metadata.issuer)
    }

    fn tls_pair_for_listen(&mut self) -> Option<(String, String)> {
        self.active_server_tls.clone()
    }
}

#[async_trait]
impl RunnableDependency for OauthDependency {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn start(&mut self) {
        if self.running {
            return;
        }

        tracing::debug!(dependency = %self.identifier, phase = "start_begin", "starting");
        let sw = Instant::now();

        if let Some(children) = self.dependencies.as_mut() {
            if !children.is_empty() {
                self.children_started = true;
                for dep in children.iter_mut() {
                    dep.start().await;
                }
            }
        }

        let tls_for_server = self.tls_pair_for_listen();

        self.needs_teardown = true;
        self.oauth_server
            .start(
                &self.identifier,
                self.listen,
                self.keys.clone(),
                self.scopes_supported.clone(),
                self.token_ttl_secs,
                tls_for_server,
                self.metadata_base_url.clone(),
            )
            .await;
        self.oauth_server.wait_until_ready(&self.identifier).await;

        self.running = true;
        tracing::debug!(
            dependency = %self.identifier,
            issuer = %self.base_url().unwrap_or(""),
            elapsed = ?sw.elapsed(),
            "started"
        );
    }

    async fn stop(&mut self) {
        self.oauth_server.stop().await;
        self.needs_teardown = false;

        if !self.running {
            if self.children_started {
                for dep in self.dependencies.iter_mut().flatten().rev() {
                    dep.stop().await;
                }
                self.children_started = false;
            }
            return;
        }

        tracing::debug!(dependency = %self.identifier, phase = "stop_begin", "stopping");

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.stop().await;
        }

        self.children_started = false;
        self.running = false;
        tracing::debug!(dependency = %self.identifier, phase = "stopped", "stopped");
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.get_or_insert_with(Vec::new).push(dep);
    }

    async fn soft_reset(&self) {
        if !self.running {
            return;
        }
        tracing::debug!(dependency = %self.identifier, phase = "soft_reset", "no-op");
    }

    async fn hard_reset(&mut self) {
        if !self.running {
            return;
        }
        tracing::debug!(dependency = %self.identifier, phase = "hard_reset", "restarting oauth server");
        self.stop().await;
        self.start().await;
    }
}

impl Drop for OauthDependency {
    fn drop(&mut self) {
        if self.running {
            tracing::warn!(
                dependency = %self.identifier,
                "drop while oauth server running; forcing stop"
            );
            futures::executor::block_on(<Self as RunnableDependency>::stop(self));
        } else if self.needs_teardown || self.children_started {
            futures::executor::block_on(<Self as RunnableDependency>::stop(self));
        }
    }
}
