pub(crate) mod container_impl;
mod healthcheck;

use crate::builder::TemporalDependencyBuilder;
use crate::temporal_dependency::healthcheck::DefaultTemporalReadinessCheck;
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use futures_timer::Delay;
use std::time::{Duration, Instant};

#[async_trait]
pub trait TemporalImpl: Send + Sync {
    async fn start(
        &mut self,
        grpc_port: u16,
        ui_port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    );
    async fn stop(&mut self);
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

    async fn wait_until_ready(&self) {
        let timeout = Duration::from_secs(30);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();

        let endpoint = loop {
            if start.elapsed() >= timeout {
                panic!(
                    "[Temporal-{}] temporal did not become ready within {:?}",
                    self.identifier, timeout
                );
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
            panic!(
                "[Temporal-{}] temporal did not become ready within {:?}",
                self.identifier, timeout
            );
        }

        match self
            .readiness_check
            .is_ready(&self.identifier, &endpoint, remaining.as_millis() as u64)
            .await
        {
            Ok(()) => {}
            Err(err) => panic!(
                "[Temporal-{}] readiness check failed: {}",
                self.identifier, err
            ),
        }
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
        self.temporal_impl
            .start(
                self.grpc_port,
                self.ui_port,
                &image_name,
                &image_tag,
                &container_name,
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
        self.temporal_impl.stop().await;
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
            "soft reset skipped: no reset primitive without a temporal client"
        );
    }

    async fn hard_reset(&mut self) {
        if !self.running {
            return;
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

        self.temporal_impl.stop().await;
        self.running = false;

        self.temporal_impl
            .start(
                self.grpc_port,
                self.ui_port,
                &image_name,
                &image_tag,
                &container_name,
            )
            .await;
        self.wait_until_ready().await;
        self.running = true;
    }
}

impl Drop for TemporalDependency {
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
