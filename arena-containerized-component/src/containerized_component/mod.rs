pub(crate) mod container_impl;

use crate::builder::ContainerizedComponentBuilder;
use arena::component::RunnableComponent;
use arena::healthcheck::ReadinessCheck;
use arena_container::mount::MountSpec;
use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait ContainerizedComponentImpl: Send + Sync {
    async fn build_image(
        &self,
        identifier: &str,
        containerfile: &str,
        image_tag: &str,
        build_context: Option<&Path>,
    );

    #[allow(clippy::too_many_arguments)]
    async fn start_container(
        &self,
        identifier: &str,
        image_tag: &str,
        network: Option<&str>,
        network_alias: Option<&str>,
        env_vars: &[(String, String)],
        runtime_args: &[(String, String)],
        port_mappings: &[(u16, u16)],
        host_mappings: &[String],
        mounts: &[MountSpec],
    ) -> String;

    fn follow_logs(&self, container_id: &str, identifier: &str);

    async fn stop_container(&self, container_id: &str, identifier: &str);
}

pub struct ContainerizedComponent {
    pub(crate) identifier: String,
    pub(crate) children: Option<Vec<Box<dyn RunnableComponent>>>,
    pub(crate) image_tag: String,
    pub(crate) network: Option<String>,
    pub(crate) network_alias: Option<String>,
    pub(crate) env_vars: Vec<(String, String)>,
    pub(crate) runtime_args: Vec<(String, String)>,
    pub(crate) port_mappings: Vec<(u16, u16)>,
    pub(crate) readiness_checks: Vec<(Box<dyn ReadinessCheck>, String, u64)>,
    pub(crate) host_mappings: Vec<String>,
    pub(crate) mounts: Vec<MountSpec>,
    pub(crate) container_impl: Box<dyn ContainerizedComponentImpl>,
    pub(crate) container_id: Option<String>,
    pub(crate) stopped: bool,
}

impl ContainerizedComponent {
    pub fn builder(
        identifier: impl Into<String>,
        containerfile: impl Into<String>,
    ) -> ContainerizedComponentBuilder {
        ContainerizedComponentBuilder::new(identifier, containerfile)
    }

    async fn wait_until_ready(&self) {
        if self.readiness_checks.is_empty() {
            return;
        }

        for (check, target, check_timeout_ms) in &self.readiness_checks {
            match check
                .is_ready(&self.identifier, target, *check_timeout_ms)
                .await {
                Ok(()) => {
                    tracing::debug!(
                        component = %self.identifier,
                        readiness_target = %target,
                        "readiness check passed",
                    );
                }
                Err(msg) => {
                    panic!(
                        "{}: readiness check failed for target {}: {}",
                        self.identifier, target, msg
                    );
                }
            }
        }
        tracing::debug!(
            component = %self.identifier,
            "all readiness checks passed",
        );
    }
}

#[async_trait]
impl RunnableComponent for ContainerizedComponent {
    async fn start(&mut self) {
        for child in self.children.iter_mut().flatten() {
            child.start().await;
        }

        tracing::debug!(
            component = %self.identifier,
            phase = "start_begin",
            "starting",
        );

        let container_id = self
            .container_impl
            .start_container(
                &self.identifier,
                &self.image_tag,
                self.network.as_deref(),
                self.network_alias.as_deref(),
                &self.env_vars,
                &self.runtime_args,
                &self.port_mappings,
                &self.host_mappings,
                &self.mounts,
            )
            .await;
        self.container_impl
            .follow_logs(&container_id, &self.identifier);
        self.container_id = Some(container_id);

        self.wait_until_ready().await;

        tracing::debug!(
            component = %self.identifier,
            phase = "start_done",
            "started",
        );
    }

    async fn stop(&mut self) {
        if self.stopped {
            return;
        }

        tracing::debug!(
            component = %self.identifier,
            phase = "stop_begin",
            "stopping",
        );

        if let Some(container_id) = self.container_id.take() {
            self.container_impl
                .stop_container(&container_id, &self.identifier)
                .await;
        }

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }

        tracing::debug!(
            component = %self.identifier,
            phase = "stop_done",
            "stopped",
        );

        for child in self.children.iter_mut().flatten().rev() {
            child.stop().await;
        }

        self.stopped = true;
    }

    fn add_child(&mut self, child: Box<dyn RunnableComponent>) {
        self.children.get_or_insert_with(Vec::new).push(child);
    }
}
