use arena::lifecycle::message;
use arena::lifecycle::Subject;
use std::net::IpAddr;
use std::time::Instant;

use arena::dependency::{Dependency, RunnableDependency};
use arena::lifecycle::{Fault, RunnableState};
use async_trait::async_trait;

use crate::builder::OauthDependencyBuilder;
use arena_cryptography::ephemeral_tls;
use crate::oauth_common::{IssuerRegistration, OauthListenAddr};
use crate::oauth_server::OauthServer;
use crate::provider::Provider;
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
    issuers: Vec<IssuerRegistration>,
    scopes_supported: Vec<String>,
    token_ttl_secs: u64,
    active_server_tls: Option<(String, String)>,
    metadata_base_url: Option<String>,
    oauth_server: OauthServer,
    state: RunnableState,
    faults: Vec<Fault>,
    build_fault: Option<Fault>,
}

pub fn ephemeral_tls_hosts(listen_ip: IpAddr) -> Vec<String> {
    let mut hosts = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    if !listen_ip.is_unspecified() {
        let listen_host = listen_ip.to_string();
        if !hosts.contains(&listen_host) {
            hosts.push(listen_host);
        }
    }
    hosts
}

impl OauthDependency {
    pub(crate) fn new(
        identifier: String,
        issuers: Vec<IssuerRegistration>,
        listen: OauthListenAddr,
        dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
        scopes_supported: Vec<String>,
        token_ttl_secs: u64,
        tls_plan: OauthTlsPlan,
        metadata_base_url: Option<String>,
    ) -> Self {
        let (active_server_tls, build_fault) = match &tls_plan {
            OauthTlsPlan::Disabled => (None, None),
            OauthTlsPlan::CustomPem { cert_pem, key_pem } => {
                (Some((cert_pem.clone(), key_pem.clone())), None)
            }
            OauthTlsPlan::EphemeralOnStart => {
                match ephemeral_tls::self_signed_pem_pair(&ephemeral_tls_hosts(listen.ip)) {
                    Ok(pair) => (Some(pair), None),
                    Err(e) => (
                        None,
                        Some(Fault::dependency(
                            &identifier,
                            format!("ephemeral TLS certificate generation failed: {e}"),
                        )),
                    ),
                }
            }
        };
        Self {
            identifier,
            listen,
            dependencies,
            running: false,
            needs_teardown: false,
            children_started: false,
            issuers,
            scopes_supported,
            token_ttl_secs,
            active_server_tls,
            metadata_base_url,
            oauth_server: OauthServer::default(),
            state: RunnableState::NotStarted,
            faults: Vec::new(),
            build_fault,
        }
    }

    pub fn builder(identifier: impl Into<String>) -> OauthDependencyBuilder {
        OauthDependencyBuilder::new(identifier)
    }

    pub fn base_url(&self) -> Option<&str> {
        self.oauth_server.base_url()
    }

    pub fn issuer(&self) -> Option<String> {
        let issuer = self.issuers.first()?;
        let base = self.base_url()?.trim_end_matches('/');
        Some(format!("{base}{}", issuer.issuer_path))
    }

    pub fn server_tls_certificate_pem(&self) -> Option<&str> {
        self.active_server_tls.as_ref().map(|(c, _)| c.as_str())
    }

    pub fn verify_access_token(&self, token: &str) -> Result<AccessTokenClaims, TokenError> {
        let state = self
            .oauth_server
            .signing_state()
            .ok_or(TokenError::NotRunning)?;
        let base = self
            .base_url()
            .ok_or(TokenError::NotRunning)?
            .trim_end_matches('/');
        let mut first_err: Option<TokenError> = None;
        for issuer in state.issuers.iter() {
            let issuer_string = format!("{base}{}", issuer.issuer_path);
            match crate::token::verify_access_token(token, issuer.keys.resolve(), &issuer_string) {
                Ok(claims) => return Ok(claims),
                Err(e) => {
                    if first_err.is_none() {
                        first_err = Some(e);
                    }
                }
            }
        }
        Err(first_err.unwrap_or(TokenError::NotRunning))
    }

    pub fn issuer_count(&self) -> usize {
        self.issuers.len()
    }

    pub fn jwks_path_at(&self, index: usize) -> Option<&str> {
        self.issuers.get(index).map(|i| i.jwks_path.as_str())
    }

    pub fn issuer_path_at(&self, index: usize) -> Option<&str> {
        self.issuers.get(index).map(|i| i.issuer_path.as_str())
    }

    pub fn issuer_for(&self, provider: &Provider) -> Option<String> {
        let issuer = self.issuers.iter().find(|i| &i.provider == provider)?;
        let base = self.base_url()?.trim_end_matches('/');
        Some(format!("{base}{}", issuer.issuer_path))
    }

    pub fn signing_key_pem_for(&self, provider: &Provider) -> Option<String> {
        self.issuers
            .iter()
            .find(|i| &i.provider == provider)
            .and_then(|issuer| issuer.keys.resolve().private_key_pkcs8_pem().ok())
    }

    pub fn sign_claims(
        &self,
        provider: &Provider,
        claims: &serde_json::Value,
    ) -> Result<String, String> {
        let issuer = self
            .issuers
            .iter()
            .find(|i| &i.provider == provider)
            .ok_or_else(|| format!("no issuer registered for provider {provider:?}"))?;
        issuer.keys.resolve().sign_claims(claims)
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

    fn state(&self) -> RunnableState {
        self.state
    }

    fn faults(&self) -> &[Fault] {
        &self.faults
    }

    async fn start(&mut self) -> Result<(), Fault> {
        if self.running {
            return Ok(());
        }
        if let Some(fault) = self.build_fault.take() {
            self.faults.push(fault.clone());
            return Err(fault);
        }
        self.state = RunnableState::Starting;

        tracing::debug!(dependency = %self.identifier, phase = "start_begin", "starting");
        let sw = Instant::now();

        if let Some(children) = self.dependencies.as_mut() {
            if !children.is_empty() {
                self.children_started = true;
                let mut child_faults = Vec::new();
                for dep in children.iter_mut() {
                    if let Err(fault) = arena::dependency::start_child(dep).await {
                        child_faults.push(fault);
                    }
                }
                if !child_faults.is_empty() {
                    return Err(self.fail(message::child_start_failed(Subject::Dependency), child_faults).await);
                }
            }
        }

        let tls_for_server = self.tls_pair_for_listen();

        self.needs_teardown = true;
        if let Err(message) = self
            .oauth_server
            .start(
                self.listen,
                self.issuers.clone(),
                self.scopes_supported.clone(),
                self.token_ttl_secs,
                tls_for_server,
                self.metadata_base_url.clone(),
            )
            .await
        {
            return Err(self.fail(message, Vec::new()).await);
        }

        self.state = RunnableState::ReadinessCheck;
        if let Err(message) = self.oauth_server.wait_until_ready(&self.identifier).await {
            return Err(self.fail(message, Vec::new()).await);
        }

        self.running = true;
        self.state = RunnableState::Started;
        tracing::debug!(
            dependency = %self.identifier,
            issuer = %self.base_url().unwrap_or(""),
            elapsed = ?sw.elapsed(),
            "started"
        );
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        self.state = RunnableState::Stopping;
        self.oauth_server.stop().await;
        self.needs_teardown = false;

        tracing::debug!(dependency = %self.identifier, phase = "stop_begin", "stopping");

        let mut causes = Vec::new();
        for dep in self.dependencies.iter_mut().flatten().rev() {
            if let Err(fault) = arena::dependency::stop_child(dep).await {
                causes.push(fault);
            }
        }

        self.children_started = false;
        self.running = false;

        if !causes.is_empty() {
            let fault =
                Fault::dependency(&self.identifier, message::stop_did_not_complete()).caused_by_all(causes);
            self.faults.push(fault.clone());
            self.state = RunnableState::Faulted;
            return Err(fault);
        }

        self.state = RunnableState::Stopped;
        tracing::debug!(dependency = %self.identifier, phase = "stopped", "stopped");
        Ok(())
    }

    fn release(&mut self) {
        self.oauth_server.release();
        self.running = false;
        self.needs_teardown = false;
        self.children_started = false;
        for dep in self.dependencies.iter_mut().flatten().rev() {
            arena::dependency::release_child(dep);
        }
        self.state = RunnableState::Stopped;
    }

    async fn force_stop(&mut self) {
        self.oauth_server.stop().await;
        self.needs_teardown = false;
        self.running = false;
        self.children_started = false;

        for dep in self.dependencies.iter_mut().flatten().rev() {
            arena::dependency::force_stop_child(dep).await;
        }

        self.state = RunnableState::Stopped;
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.get_or_insert_with(Vec::new).push(dep);
    }

    fn children(&self) -> &[Dependency] {
        self.dependencies.as_deref().unwrap_or(&[])
    }

    fn children_mut(&mut self) -> &mut [Dependency] {
        self.dependencies.as_deref_mut().unwrap_or(&mut [])
    }

    async fn soft_reset(&self) -> Result<(), Fault> {
        if !self.running {
            return Ok(());
        }
        tracing::debug!(dependency = %self.identifier, phase = "soft_reset", "no-op");
        Ok(())
    }

    async fn hard_reset(&mut self) -> Result<(), Fault> {
        if !self.running {
            return Ok(());
        }
        tracing::debug!(dependency = %self.identifier, phase = "hard_reset", "restarting oauth server");
        if let Err(fault) = self.stop().await {
            return Err(self.fail("hard reset could not stop the oauth server", vec![fault])
                .await);
        }
        self.start().await
    }
}

impl OauthDependency {
    async fn fail(&mut self, message: impl Into<String>, causes: Vec<Fault>) -> Fault {
        let fault = Fault::dependency(&self.identifier, message).caused_by_all(causes);
        self.faults.push(fault.clone());
        <Self as RunnableDependency>::force_stop(self).await;
        fault
    }
}

impl Drop for OauthDependency {
    fn drop(&mut self) {
        if self.running || self.needs_teardown || self.children_started {
            tracing::warn!(
                dependency = %self.identifier,
                "drop while oauth server running; releasing server"
            );
            self.oauth_server.release();
            self.running = false;
            self.needs_teardown = false;
            self.children_started = false;
            self.state = RunnableState::Stopped;
        }
    }
}
