pub(crate) mod container_impl;
mod healthcheck;

use arena::lifecycle::message;
use arena::lifecycle::Subject;
use crate::builder::TemporalDependencyBuilder;
use crate::temporal_dependency::healthcheck::DefaultTemporalReadinessCheck;
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena::lifecycle::{Fault, RunnableState};
use async_trait::async_trait;
use futures_timer::Delay;
use std::time::{Duration, Instant};

#[async_trait]
pub trait TemporalImpl: Send + Sync {
    fn set_expiry(&mut self, _expiry: Option<Duration>) {}
    async fn start(
        &mut self,
        grpc_port: u16,
        ui_port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    ) -> Result<(), String>;
    async fn stop(&mut self) -> Result<(), String>;
    async fn force_stop(&mut self) -> bool;
    fn release(&mut self);
    fn grpc_endpoint(&self) -> Option<&str>;
    fn ui_url(&self) -> Option<&str>;
}

pub struct TemporalDependency {
    pub identifier: String,
    temporal_impl: Box<dyn TemporalImpl>,
    grpc_port: u16,
    ui_port: u16,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    running: bool,
    needs_teardown: bool,
    children_started: bool,
    image_name: String,
    image_tag: String,
    container_name: Option<String>,
    readiness_check: Box<dyn ReadinessCheck>,
    state: RunnableState,
    faults: Vec<Fault>,
}

impl TemporalDependency {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        identifier: String,
        temporal_impl: Box<dyn TemporalImpl>,
        grpc_port: u16,
        ui_port: u16,
        dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
        image_name: String,
        image_tag: String,
        container_name: Option<String>,
    ) -> Self {
        Self {
            identifier,
            temporal_impl,
            grpc_port,
            ui_port,
            dependencies,
            image_name,
            image_tag,
            container_name,
            running: false,
            needs_teardown: false,
            children_started: false,
            readiness_check: Box::new(DefaultTemporalReadinessCheck),
            state: RunnableState::NotStarted,
            faults: Vec::new(),
        }
    }

    pub fn grpc_endpoint(&self) -> Option<&str> {
        self.temporal_impl.grpc_endpoint()
    }

    pub fn ui_url(&self) -> Option<&str> {
        self.temporal_impl.ui_url()
    }

    pub fn builder(identifier: impl Into<String>) -> TemporalDependencyBuilder {
        TemporalDependencyBuilder::new(identifier)
    }

    pub(crate) fn set_readiness_check(&mut self, check: Box<dyn ReadinessCheck>) {
        self.readiness_check = check;
    }

    fn grpc_endpoint_on_host(&self) -> Result<&str, String> {
        self.grpc_endpoint()
            .ok_or_else(|| "temporal grpc endpoint not available yet".to_string())
    }

    async fn wait_until_ready(&self) -> Result<(), String> {
        let timeout = Duration::from_secs(30);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();

        let endpoint = loop {
            if start.elapsed() >= timeout {
                return Err(format!(
                    "temporal did not become ready within {timeout:?}"
                ));
            }

            match self.grpc_endpoint_on_host() {
                Ok(v) => break v.to_string(),
                Err(err) => {
                    tracing::debug!(
                        dependency = %self.identifier,
                        reason = %err,
                        "temporal grpc endpoint not ready yet"
                    );
                    Delay::new(poll_every).await;
                }
            }
        };

        let remaining = timeout.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            return Err(format!("temporal did not become ready within {timeout:?}"));
        }

        self.readiness_check
            .is_ready(&self.identifier, &endpoint, remaining.as_millis() as u64)
            .await
            .map_err(message::readiness_failed)
    }

    async fn fail(&mut self, message: impl Into<String>, causes: Vec<Fault>) -> Fault {
        let fault = Fault::dependency(&self.identifier, message).caused_by_all(causes);
        self.faults.push(fault.clone());
        <Self as RunnableDependency>::force_stop(self).await;
        fault
    }
}

#[async_trait]
impl RunnableDependency for TemporalDependency {
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

        tracing::debug!(dependency = %self.identifier, phase = "start_begin", "starting");
        let sw = Instant::now();
        self.state = RunnableState::Starting;

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
            .temporal_impl
            .start(
                self.grpc_port,
                self.ui_port,
                &image_name,
                &image_tag,
                &container_name,
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

        if let Err(message) = self.temporal_impl.stop().await {
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
            let fault = Fault::dependency(&self.identifier, message::stop_did_not_complete())
                .caused_by_all(causes);
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
        self.temporal_impl.release();
        self.running = false;
        self.needs_teardown = false;
        self.children_started = false;
        for dep in self.dependencies.iter_mut().flatten().rev() {
            arena::dependency::release_child(dep);
        }
        self.state = RunnableState::Stopped;
    }

    async fn force_stop(&mut self) {
        let removed = self.temporal_impl.force_stop().await;
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

        tracing::warn!(
            dependency = %self.identifier,
            "soft reset skipped: no reset primitive without a temporal client"
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
            "restarting temporal container"
        );

        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = arena_container::identifier::resolve_container_name(
            &self.identifier,
            self.container_name.as_deref(),
        );

        if let Err(message) = self.temporal_impl.stop().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        self.running = false;

        if let Err(message) = self
            .temporal_impl
            .start(
                self.grpc_port,
                self.ui_port,
                &image_name,
                &image_tag,
                &container_name,
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

impl Drop for TemporalDependency {
    fn drop(&mut self) {
        if self.running || self.needs_teardown || self.children_started {
            tracing::warn!(
                dependency = %self.identifier,
                "drop while running; releasing container"
            );
            self.temporal_impl.release();
            self.running = false;
            self.needs_teardown = false;
            self.children_started = false;
            self.state = RunnableState::Stopped;
        }
    }
}
