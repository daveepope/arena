pub(crate) mod container_impl;
mod healthcheck;

use crate::builder::SmtpDependencyBuilder;
use crate::smtp_dependency::healthcheck::DefaultSmtpReadinessCheck;
use arena::dependency::{Dependency, RunnableDependency};
use arena::lifecycle::{Fault, RunnableState};
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
    fn set_expiry(&mut self, _expiry: Option<Duration>) {}
    async fn start(
        &mut self,
        smtp_port: u16,
        ui_port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
        tls: Option<&SmtpTlsConfig>,
    ) -> Result<(), String>;
    async fn stop(&mut self) -> Result<(), String>;
    async fn force_stop(&mut self) -> bool;
    fn release(&mut self);
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
    state: RunnableState,
    faults: Vec<Fault>,
    build_fault: Option<Fault>,
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
        let (active_tls, build_fault) = match tls_mode {
            None => (None, None),
            Some(mode) => {
                match arena_cryptography::ephemeral_tls::localhost_self_signed_pem_pair() {
                    Ok((certificate_pem, private_key_pem)) => (
                        Some(SmtpTlsConfig {
                            mode,
                            certificate_pem,
                            private_key_pem,
                        }),
                        None,
                    ),
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
        let implicit_tls = matches!(tls_mode, Some(SmtpTlsMode::Implicit));
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
            state: RunnableState::NotStarted,
            faults: Vec::new(),
            build_fault,
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

    async fn wait_until_ready(&self) -> Result<(), String> {
        let timeout = Duration::from_secs(30);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();

        let address = loop {
            if start.elapsed() >= timeout {
                return Err(format!("smtp did not become ready within {timeout:?}"));
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
            return Err(format!("smtp did not become ready within {timeout:?}"));
        }

        self.readiness_check
            .is_ready(&self.identifier, &address, remaining.as_millis() as u64)
            .await
            .map_err(|err| format!("readiness check failed: {err}"))
    }

    async fn fail(&mut self, message: impl Into<String>, causes: Vec<Fault>) -> Fault {
        let fault = Fault::dependency(&self.identifier, message).caused_by_all(causes);
        self.faults.push(fault.clone());
        <Self as RunnableDependency>::force_stop(self).await;
        fault
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
                    if let Err(fault) = dep.start().await {
                        child_faults.push(fault);
                    }
                }
                if !child_faults.is_empty() {
                    return Err(self.fail("child dependency failed to start", child_faults).await);
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
        if let Err(message) =         self.smtp_impl
            .start(
                self.smtp_port,
                self.ui_port,
                &image_name,
                &image_tag,
                &container_name,
                self.active_tls.as_ref(),
            )
            .await
        {
            return Err(self.fail(message, Vec::new()).await);
        }
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_container.elapsed(),
            "container start finished"
        );

        let sw_ready = Instant::now();
        self.state = RunnableState::ReadinessCheck;
        if let Err(message) = self.wait_until_ready().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_ready.elapsed(),
            "readiness wait finished"
        );

        self.running = true;
        self.state = RunnableState::Started;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "started"
        );
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        self.state = RunnableState::Stopping;
        let mut causes = Vec::new();

        if let Err(message) = self.smtp_impl.stop().await {
            causes.push(Fault::dependency(&self.identifier, message));
        }
        self.needs_teardown = false;

        tracing::debug!(dependency = %self.identifier, phase = "stop_begin", "stopping");
        let sw = Instant::now();

        for dep in self.dependencies.iter_mut().flatten().rev() {
            if let Err(fault) = dep.stop().await {
                causes.push(fault);
            }
        }

        self.children_started = false;
        self.running = false;

        if !causes.is_empty() {
            let fault =
                Fault::dependency(&self.identifier, "stop did not complete").caused_by_all(causes);
            self.faults.push(fault.clone());
            self.state = RunnableState::Faulted;
            return Err(fault);
        }

        self.state = RunnableState::Stopped;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "stopped"
        );
        Ok(())
    }

    fn release(&mut self) {
        self.smtp_impl.release();
        self.running = false;
        self.needs_teardown = false;
        self.children_started = false;
        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.release();
        }
        self.state = RunnableState::Stopped;
    }

    async fn force_stop(&mut self) {
        let removed = self.smtp_impl.force_stop().await;
        self.needs_teardown = false;
        self.running = false;
        self.children_started = false;

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.force_stop().await;
        }

        if removed {
            self.state = RunnableState::Stopped;
            return;
        }

        let unconfirmed = Fault::dependency(
            &self.identifier,
            "forced teardown could not confirm the container was removed",
        );
        if !self
            .faults
            .iter()
            .any(|f| f.message == unconfirmed.message && f.id == unconfirmed.id)
        {
            self.faults.push(unconfirmed);
        }
        self.state = RunnableState::Faulted;
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

        tracing::warn!(
            dependency = %self.identifier,
            "soft reset skipped: no reset primitive without an smtp client"
        );
        Ok(())
    }

    async fn hard_reset(&mut self) -> Result<(), Fault> {
        if !self.running {
            return Ok(());
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

        if let Err(message) = self.smtp_impl.stop().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        self.running = false;

        if let Err(message) = self
            .smtp_impl
            .start(
                self.smtp_port,
                self.ui_port,
                &image_name,
                &image_tag,
                &container_name,
                self.active_tls.as_ref(),
            )
            .await
        {
            return Err(self.fail(message, Vec::new()).await);
        }
        if let Err(message) = self.wait_until_ready().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        self.running = true;
        self.state = RunnableState::Started;
        Ok(())
    }
}

impl Drop for SmtpDependency {
    fn drop(&mut self) {
        if self.running || self.needs_teardown || self.children_started {
            tracing::warn!(
                dependency = %self.identifier,
                "drop while running; releasing container"
            );
            self.smtp_impl.release();
            self.running = false;
            self.needs_teardown = false;
            self.children_started = false;
            self.state = RunnableState::Stopped;
        }
    }
}
