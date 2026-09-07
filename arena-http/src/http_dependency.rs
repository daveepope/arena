pub(crate) mod container_impl;
mod healthcheck;

use arena::lifecycle::message;
use arena::lifecycle::Subject;
use crate::admin_client::admin_api_client;
use crate::builder::HttpDependencyBuilder;
use crate::http_dependency::healthcheck::DefaultHttpReadinessCheck;
use arena::dependency::{Dependency, RunnableDependency};
use arena::lifecycle::{Fault, RunnableState};
use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use std::time::{Duration, Instant};

#[async_trait]
pub trait HttpImpl: Send + Sync {
    fn set_expiry(&mut self, _expiry: Option<Duration>) {}
    async fn start(
        &mut self,
        port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    ) -> Result<(), String>;
    async fn stop(&mut self) -> Result<(), String>;
    async fn force_stop(&mut self) -> bool;
    fn release(&mut self);
    fn base_url(&self) -> Option<&str>;
    fn admin_url(&self) -> Option<String>;
    fn https_base_url(&self) -> Option<&str> {
        None
    }
}

pub struct HttpDependency {
    pub identifier: String,
    http_impl: Box<dyn HttpImpl>,
    port: u16,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    running: bool,
    needs_teardown: bool,
    children_started: bool,
    image_name: String,
    image_tag: String,
    container_name: Option<String>,
    trusted_tls_certificate_pem: Option<String>,
    readiness_check: Box<dyn ReadinessCheck>,
    state: RunnableState,
    faults: Vec<Fault>,
}

impl HttpDependency {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        identifier: String,
        http_impl: Box<dyn HttpImpl>,
        port: u16,
        dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
        image_name: String,
        image_tag: String,
        container_name: Option<String>,
        trusted_tls_certificate_pem: Option<String>,
    ) -> Self {
        let readiness_check: Box<dyn ReadinessCheck> = Box::new(DefaultHttpReadinessCheck::new(
            trusted_tls_certificate_pem.clone(),
        ));
        Self {
            identifier,
            http_impl,
            port,
            dependencies,
            image_name,
            image_tag,
            container_name,
            trusted_tls_certificate_pem,
            running: false,
            needs_teardown: false,
            children_started: false,
            readiness_check,
            state: RunnableState::NotStarted,
            faults: Vec::new(),
        }
    }

    pub fn base_url(&self) -> Option<&str> {
        self.http_impl.base_url()
    }

    pub fn admin_url(&self) -> Option<String> {
        self.http_impl.admin_url()
    }

    pub fn https_base_url(&self) -> Option<&str> {
        self.http_impl.https_base_url()
    }

    pub fn trusted_certificate_pem(&self) -> Option<&str> {
        self.trusted_tls_certificate_pem.as_deref()
    }

    pub fn builder(identifier: impl Into<String>) -> HttpDependencyBuilder {
        HttpDependencyBuilder::new(identifier)
    }

    pub fn playbook(&self) -> crate::playbook::Playbook {
        crate::playbook::Playbook::with(self)
    }

    pub async fn reset_journal(&self) -> Result<(), Fault> {
        if !self.running {
            return Ok(());
        }

        let admin_url = self
            .admin_url()
            .ok_or_else(|| Fault::dependency(&self.identifier, "admin url not available yet"))?;

        tracing::debug!(
            dependency = %self.identifier,
            phase = "reset_journal",
            "clearing request journal"
        );

        let client = admin_api_client(&admin_url, self.trusted_tls_certificate_pem.as_deref())
            .map_err(|message| Fault::dependency(&self.identifier, message))?;
        let response = client
            .delete(format!("{admin_url}/requests"))
            .send()
            .await
            .map_err(|e| {
                Fault::dependency(&self.identifier, format!("reset journal failed: {e}"))
            })?;

        if !response.status().is_success() {
            return Err(Fault::dependency(
                &self.identifier,
                format!("reset journal got HTTP {}", response.status()),
            ));
        }
        Ok(())
    }

    pub(crate) fn set_readiness_check(&mut self, check: Box<dyn ReadinessCheck>) {
        self.readiness_check = check;
    }

    async fn fail(&mut self, message: impl Into<String>, causes: Vec<Fault>) -> Fault {
        let fault = Fault::dependency(&self.identifier, message).caused_by_all(causes);
        self.faults.push(fault.clone());
        <Self as RunnableDependency>::force_stop(self).await;
        fault
    }

    async fn wait_until_ready(&self) -> Result<(), String> {
        let timeout = Duration::from_secs(45);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();

        let admin_url = loop {
            if start.elapsed() >= timeout {
                return Err(format!("did not become ready within {timeout:?}"));
            }

            match self.admin_url() {
                Some(url) => break url,
                None => {
                    tracing::debug!(
                        dependency = %self.identifier,
                        phase = "readiness_poll",
                        "admin url not available yet"
                    );
                    futures_timer::Delay::new(poll_every).await;
                }
            }
        };

        let remaining = timeout.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            return Err(format!("did not become ready within {timeout:?}"));
        }

        self.readiness_check
            .is_ready(&self.identifier, &admin_url, remaining.as_millis() as u64)
            .await
            .map_err(message::readiness_failed)
    }
}

#[async_trait]
impl RunnableDependency for HttpDependency {
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

        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = arena_container::identifier::resolve_container_name(
            &self.identifier,
            self.container_name.as_deref(),
        );

        let sw_container = Instant::now();
        self.needs_teardown = true;
        if let Err(message) = self
            .http_impl
            .start(self.port, &image_name, &image_tag, &container_name)
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

        if let Err(message) = self.http_impl.stop().await {
            causes.push(Fault::dependency(&self.identifier, message));
        }
        self.needs_teardown = false;

        tracing::debug!(dependency = %self.identifier, phase = "stop_begin", "stopping");
        let sw = Instant::now();

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
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "stopped"
        );
        Ok(())
    }

    fn release(&mut self) {
        self.http_impl.release();
        self.running = false;
        self.needs_teardown = false;
        self.children_started = false;
        for dep in self.dependencies.iter_mut().flatten().rev() {
            arena::dependency::release_child(dep);
        }
        self.state = RunnableState::Stopped;
    }

    async fn force_stop(&mut self) {
        let removed = self.http_impl.force_stop().await;
        self.needs_teardown = false;
        self.running = false;
        self.children_started = false;

        for dep in self.dependencies.iter_mut().flatten().rev() {
            arena::dependency::force_stop_child(dep).await;
        }

        if removed {
            self.state = RunnableState::Stopped;
            return;
        }

        let unconfirmed = Fault::dependency(
            &self.identifier,
            message::forced_teardown_unconfirmed(),
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

        let admin_url = self
            .admin_url()
            .ok_or_else(|| Fault::dependency(&self.identifier, "admin url not available yet"))?;

        tracing::debug!(
            dependency = %self.identifier,
            phase = "soft_reset",
            "reset mappings and request journal"
        );

        let client = admin_api_client(&admin_url, self.trusted_tls_certificate_pem.as_deref())
            .map_err(|message| Fault::dependency(&self.identifier, message))?;
        client
            .post(format!("{admin_url}/reset"))
            .send()
            .await
            .map_err(|e| Fault::dependency(&self.identifier, format!("soft reset failed: {e}")))?;
        Ok(())
    }

    async fn hard_reset(&mut self) -> Result<(), Fault> {
        if !self.running {
            return Ok(());
        }

        tracing::debug!(
            dependency = %self.identifier,
            phase = "hard_reset",
            "restarting http container"
        );

        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = arena_container::identifier::resolve_container_name(
            &self.identifier,
            self.container_name.as_deref(),
        );

        if let Err(message) = self.http_impl.stop().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        self.running = false;

        if let Err(message) = self
            .http_impl
            .start(self.port, &image_name, &image_tag, &container_name)
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

impl Drop for HttpDependency {
    fn drop(&mut self) {
        if self.running || self.needs_teardown || self.children_started {
            tracing::warn!(
                dependency = %self.identifier,
                "drop while running; releasing container"
            );
            self.http_impl.release();
            self.running = false;
            self.needs_teardown = false;
            self.children_started = false;
            self.state = RunnableState::Stopped;
        }
    }
}
