pub(crate) mod container_impl;
mod healthcheck;

use crate::builder::SmtpDependencyBuilder;
use crate::smtp_dependency::healthcheck::DefaultSmtpReadinessCheck;
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use futures_timer::Delay;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpTlsMode {
    StartTls,
    Implicit,
}

#[derive(Debug, Clone)]
pub struct SmtpTlsConfig {
    pub mode: SmtpTlsMode,
    pub certificate_pem: String,
    pub private_key_pem: String,
}

#[async_trait]
pub trait SmtpImpl: Send + Sync {
    async fn start(
        &mut self,
        smtp_port: u16,
        ui_port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
        tls: Option<&SmtpTlsConfig>,
    );
    async fn stop(&mut self);
    fn smtp_address(&self) -> Option<&str>;
    fn http_api_url(&self) -> Option<&str>;
}

pub struct SmtpDependency {
    pub identifier: String,
    smtp_impl: Box<dyn SmtpImpl>,
    smtp_port: u16,
    ui_port: u16,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    running: bool,
    needs_teardown: bool,
    children_started: bool,
    image_name: String,
    image_tag: String,
    container_name: Option<String>,
    active_tls: Option<SmtpTlsConfig>,
    readiness_check: Box<dyn ReadinessCheck>,
}

impl SmtpDependency {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        identifier: String,
        smtp_impl: Box<dyn SmtpImpl>,
        smtp_port: u16,
        ui_port: u16,
        dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
        image_name: String,
        image_tag: String,
        container_name: Option<String>,
        tls_mode: Option<SmtpTlsMode>,
    ) -> Self {
        let active_tls = tls_mode.map(|mode| {
            let (certificate_pem, private_key_pem) =
                arena_cryptography::ephemeral_tls::localhost_self_signed_pem_pair().unwrap_or_else(
                    |e| panic!("[Smtp-{identifier}] ephemeral TLS certificate generation failed: {e}"),
                );
            SmtpTlsConfig {
                mode,
                certificate_pem,
                private_key_pem,
            }
        });
        let implicit_tls = matches!(
            active_tls.as_ref().map(|tls| tls.mode),
            Some(SmtpTlsMode::Implicit)
        );
        Self {
            identifier,
            smtp_impl,
            smtp_port,
            ui_port,
            dependencies,
            image_name,
            image_tag,
            container_name,
            active_tls,
            running: false,
            needs_teardown: false,
            children_started: false,
            readiness_check: Box::new(DefaultSmtpReadinessCheck::new(implicit_tls)),
        }
    }

    pub fn smtp_address(&self) -> Option<&str> {
        self.smtp_impl.smtp_address()
    }

    pub fn http_api_url(&self) -> Option<&str> {
        self.smtp_impl.http_api_url()
    }

    pub fn builder(identifier: impl Into<String>) -> SmtpDependencyBuilder {
        SmtpDependencyBuilder::new(identifier)
    }

    pub(crate) fn set_readiness_check(&mut self, check: Box<dyn ReadinessCheck>) {
        self.readiness_check = check;
    }

    fn smtp_address_on_host(&self) -> Result<&str, String> {
        self.smtp_address()
            .ok_or_else(|| "smtp address not available yet".to_string())
    }

    async fn wait_until_ready(&self) {
        let timeout = Duration::from_secs(30);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();

        let address = loop {
            if start.elapsed() >= timeout {
                panic!(
                    "[Smtp-{}] smtp did not become ready within {:?}",
                    self.identifier, timeout
                );
            }

            match self.smtp_address_on_host() {
                Ok(v) => break v.to_string(),
                Err(err) => {
                    tracing::debug!(
                        dependency = %self.identifier,
                        reason = %err,
                        "smtp address not ready yet"
                    );
                    Delay::new(poll_every).await;
                }
            }
        };

        let remaining = timeout.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            panic!(
                "[Smtp-{}] smtp did not become ready within {:?}",
                self.identifier, timeout
            );
        }

        match self
            .readiness_check
            .is_ready(&self.identifier, &address, remaining.as_millis() as u64)
            .await
        {
            Ok(()) => {}
            Err(err) => panic!("[Smtp-{}] readiness check failed: {}", self.identifier, err),
        }
    }
}

#[async_trait]
impl RunnableDependency for SmtpDependency {
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

        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = arena_container::identifier::resolve_container_name(
            &self.identifier,
            self.container_name.as_deref(),
        );

        let sw_container = Instant::now();
        self.needs_teardown = true;
        self.smtp_impl
            .start(
                self.smtp_port,
                self.ui_port,
                &image_name,
                &image_tag,
                &container_name,
                self.active_tls.as_ref(),
            )
            .await;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_container.elapsed(),
            "container start finished"
        );

        let sw_ready = Instant::now();
        self.wait_until_ready().await;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_ready.elapsed(),
            "readiness wait finished"
        );

        self.running = true;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "started"
        );
    }

    async fn stop(&mut self) {
        self.smtp_impl.stop().await;
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
        let sw = Instant::now();

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.stop().await;
        }

        self.children_started = false;
        self.running = false;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "stopped"
        );
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

    async fn soft_reset(&self) {
        if !self.running {
            return;
        }

        tracing::warn!(
            dependency = %self.identifier,
            "soft reset skipped: no reset primitive without an smtp client"
        );
    }

    async fn hard_reset(&mut self) {
        if !self.running {
            return;
        }

        tracing::debug!(
            dependency = %self.identifier,
            phase = "hard_reset",
            "restarting smtp container"
        );

        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = arena_container::identifier::resolve_container_name(
            &self.identifier,
            self.container_name.as_deref(),
        );

        self.smtp_impl.stop().await;
        self.running = false;

        self.smtp_impl
            .start(
                self.smtp_port,
                self.ui_port,
                &image_name,
                &image_tag,
                &container_name,
                self.active_tls.as_ref(),
            )
            .await;
        self.wait_until_ready().await;
        self.running = true;
    }
}

impl Drop for SmtpDependency {
    fn drop(&mut self) {
        if self.running {
            tracing::warn!(
                dependency = %self.identifier,
                "drop while running; forcing stop"
            );
            futures::executor::block_on(<Self as RunnableDependency>::stop(self));
        } else if self.needs_teardown || self.children_started {
            futures::executor::block_on(<Self as RunnableDependency>::stop(self));
        }
    }
}
