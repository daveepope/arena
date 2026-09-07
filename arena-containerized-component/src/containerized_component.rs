use arena::lifecycle::message;
use arena::lifecycle::Subject;
use crate::builder::ContainerizedComponentBuilder;
use arena::component::RunnableComponent;
use arena::component::Component;
use arena::healthcheck::ReadinessCheck;
use arena::lifecycle::{Fault, RunnableState};
use async_trait::async_trait;
use bollard::container::LogOutput;
use bollard::models::{
    ContainerCreateBody, EndpointSettings, HostConfig, NetworkingConfig, PortBinding,
};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, LogsOptionsBuilder, RemoveContainerOptionsBuilder,
    StopContainerOptionsBuilder,
};
use bollard::Docker;
use futures::StreamExt;
use std::collections::HashMap;

pub struct ContainerizedComponent {
    pub(crate) identifier: String,
    pub(crate) expiry: Option<std::time::Duration>,
    pub(crate) children: Option<Vec<Box<dyn RunnableComponent>>>,
    pub(crate) image_tag: String,
    pub(crate) network: Option<String>,
    pub(crate) network_alias: Option<String>,
    pub(crate) env_vars: Vec<(String, String)>,
    pub(crate) runtime_args: Vec<(String, String)>,
    pub(crate) port_mappings: Vec<(u16, u16)>,
    pub(crate) readiness_checks: Vec<(Box<dyn ReadinessCheck>, String, u64)>,
    pub(crate) host_mappings: Vec<String>,
    pub(crate) volume_mappings: Vec<(String, String)>,
    pub(crate) runtime_client: Docker,
    pub(crate) container_id: Option<String>,
    pub(crate) stopped: bool,
    pub(crate) state: RunnableState,
    pub(crate) faults: Vec<Fault>,
}

impl ContainerizedComponent {
    pub fn builder(
        identifier: impl Into<String>,
        containerfile: impl Into<String>,
    ) -> ContainerizedComponentBuilder {
        ContainerizedComponentBuilder::new(identifier, containerfile)
    }

    pub fn from_image(
        identifier: impl Into<String>,
        image: impl Into<String>,
    ) -> ContainerizedComponentBuilder {
        ContainerizedComponentBuilder::new_from_image(identifier, image)
    }

    async fn create_and_start_container(&mut self) -> Result<(), String> {
        arena_container::expiry::remove_expired_containers_if_enabled(crate::MODULE, self.expiry)
            .await;

        arena_container::container::try_remove_existing_container(&self.identifier).await;

        tracing::debug!(
            component = %self.identifier,
            image = %self.image_tag,
            phase = "container_create_begin",
            "creating container from image",
        );

        let env: Vec<String> = self
            .env_vars
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let cmd: Vec<String> = self.runtime_args.iter().map(|(_k, v)| v.clone()).collect();

        let mut host_config = HostConfig {
            ..Default::default()
        };

        let mut exposed_ports: Vec<String> = Vec::new();
        let mut port_bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();

        for (host_port, container_port) in &self.port_mappings {
            let port_key = format!("{}/tcp", container_port);
            exposed_ports.push(port_key.clone());

            let binding = PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(host_port.to_string()),
            };
            port_bindings
                .entry(port_key)
                .or_insert_with(|| Some(Vec::new()))
                .as_mut()
                .unwrap()
                .push(binding);
        }

        if !port_bindings.is_empty() {
            host_config.port_bindings = Some(port_bindings);
        }

        if !self.host_mappings.is_empty() {
            host_config.extra_hosts = Some(self.host_mappings.clone());
        }

        if !self.volume_mappings.is_empty() {
            host_config.binds = Some(
                self.volume_mappings
                    .iter()
                    .map(|(host_path, container_path)| format!("{host_path}:{container_path}"))
                    .collect(),
            );
        }

        let mut networking_config: Option<NetworkingConfig> = None;

        if let Some(ref network) = self.network {
            arena_container::network::ensure_network_exists(network).await;

            host_config.network_mode = Some(network.clone());

            let endpoint_config = EndpointSettings {
                aliases: self.network_alias.as_ref().map(|a| vec![a.clone()]),
                ..Default::default()
            };

            let mut endpoints = HashMap::new();
            endpoints.insert(network.clone(), endpoint_config);

            networking_config = Some(NetworkingConfig {
                endpoints_config: Some(endpoints),
            });
        }

        let body = ContainerCreateBody {
            image: Some(self.image_tag.clone()),
            env: if env.is_empty() { None } else { Some(env) },
            cmd: if cmd.is_empty() { None } else { Some(cmd) },
            exposed_ports: if exposed_ports.is_empty() {
                None
            } else {
                Some(exposed_ports)
            },
            host_config: Some(host_config),
            networking_config,
            labels: Some(
                arena_container::expiry::expiry_labels_for(crate::MODULE, self.expiry)
                    .into_iter()
                    .collect(),
            ),
            ..Default::default()
        };

        let options = CreateContainerOptionsBuilder::default()
            .name(&arena_container::identifier::sanitize_for_container(
                &self.identifier,
            ))
            .build();

        let response = self
            .runtime_client
            .create_container(Some(options), body)
            .await
            .map_err(|e| format!("failed to create container: {e}"))?;

        self.container_id = Some(response.id.clone());
        let id_short = response.id[..12.min(response.id.len())].to_string();

        if self.network.is_some() {
            tracing::debug!(
                component = %self.identifier,
                network = %self.network.as_deref().unwrap_or(""),
                container_id_prefix = %id_short,
                phase = "container_created",
                "container created with network attachment",
            );
        } else {
            tracing::debug!(
                component = %self.identifier,
                container_id_prefix = %id_short,
                phase = "container_created",
                "container created",
            );
        }

        self.runtime_client
            .start_container(
                &response.id,
                None::<bollard::query_parameters::StartContainerOptions>,
            )
            .await
            .map_err(|e| format!("failed to start container: {e}"))?;

        tracing::debug!(
            component = %self.identifier,
            container_id_prefix = %id_short,
            phase = "container_running",
            "container started",
        );
        Ok(())
    }

    fn spawn_log_follower(&self) {
        let container_id = match &self.container_id {
            Some(id) => id.clone(),
            None => return,
        };
        let identifier = self.identifier.clone();
        let runtime_client = self.runtime_client.clone();

        tokio::spawn(async move {
            let options = LogsOptionsBuilder::default()
                .follow(true)
                .stdout(true)
                .stderr(true)
                .build();

            let mut stream = runtime_client.logs(&container_id, Some(options));

            while let Some(result) = stream.next().await {
                match result {
                    Ok(output) => {
                        let line = match output {
                            LogOutput::StdOut { message } => {
                                String::from_utf8_lossy(&message).trim_end().to_string()
                            }
                            LogOutput::StdErr { message } => {
                                String::from_utf8_lossy(&message).trim_end().to_string()
                            }
                            _ => continue,
                        };
                        if !line.is_empty() {
                            Self::log_line(&identifier, &line);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            component = %identifier,
                            error = %e,
                            "container log stream error",
                        );
                        break;
                    }
                }
            }
        });
    }

    fn log_line(identifier: &str, line: &str) {
        if line.contains(" ERROR ") {
            tracing::error!(component = %identifier, "{}", line);
        } else if line.contains(" WARN ") {
            tracing::warn!(component = %identifier, "{}", line);
        } else if line.contains(" DEBUG ") {
            tracing::debug!(component = %identifier, "{}", line);
        } else if line.contains(" TRACE ") {
            tracing::trace!(component = %identifier, "{}", line);
        } else {
            tracing::debug!(component = %identifier, "{}", line);
        }
    }

    async fn wait_until_ready(&self) -> Result<(), String> {
        if self.readiness_checks.is_empty() {
            return Ok(());
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
                    return Err(message::readiness_failed_for_target(target, msg));
                }
            }
        }
        tracing::debug!(
            component = %self.identifier,
            "all readiness checks passed",
        );
        Ok(())
    }

    async fn fail(&mut self, message: impl Into<String>, causes: Vec<Fault>) -> Fault {
        let fault = Fault::component(&self.identifier, message).caused_by_all(causes);
        self.faults.push(fault.clone());
        <Self as RunnableComponent>::force_stop(self).await;
        fault
    }

    async fn force_remove_container(&self) -> bool {
        let Some(container_id) = self.container_id.as_deref() else {
            return true;
        };

        let remove_options = RemoveContainerOptionsBuilder::default().force(true).build();
        if let Err(e) = self
            .runtime_client
            .remove_container(container_id, Some(remove_options))
            .await
        {
            tracing::warn!(
                component = %self.identifier,
                error = %e,
                phase = "container_force_remove",
                "forced container remove returned error",
            );
        }

        !arena_container::container::is_container_running(container_id).await
    }

    async fn stop_container(&self) {
        let container_id = match &self.container_id {
            Some(id) => id,
            None => return,
        };

        let id_short = &container_id[..12.min(container_id.len())];

        tracing::debug!(
            component = %self.identifier,
            container_id_prefix = %id_short,
            phase = "container_stop_begin",
            "stopping container",
        );

        let stop_options = StopContainerOptionsBuilder::default().t(10).build();
        if let Err(e) = self
            .runtime_client
            .stop_container(container_id, Some(stop_options))
            .await
        {
            tracing::warn!(
                component = %self.identifier,
                error = %e,
                phase = "container_stop",
                "stop container returned error",
            );
        }

        let remove_options = RemoveContainerOptionsBuilder::default().force(true).build();
        if let Err(e) = self
            .runtime_client
            .remove_container(container_id, Some(remove_options))
            .await
        {
            tracing::warn!(
                component = %self.identifier,
                error = %e,
                phase = "container_remove",
                "remove container returned error",
            );
        }

        tracing::debug!(
            component = %self.identifier,
            phase = "container_removed",
            "container removed",
        );
    }
}

#[async_trait]
impl RunnableComponent for ContainerizedComponent {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn state(&self) -> RunnableState {
        self.state
    }

    fn faults(&self) -> &[Fault] {
        &self.faults
    }

    async fn start(&mut self) -> Result<(), Fault> {
        self.state = RunnableState::Starting;

        let mut child_faults = Vec::new();
        for child in self.children.iter_mut().flatten() {
            if let Err(fault) = arena::component::start_child(child).await {
                child_faults.push(fault);
            }
        }
        if !child_faults.is_empty() {
            return Err(self.fail(message::child_start_failed(Subject::Component), child_faults).await);
        }

        tracing::debug!(
            component = %self.identifier,
            phase = "start_begin",
            "starting",
        );

        if let Err(message) = self.create_and_start_container().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        self.spawn_log_follower();

        self.state = RunnableState::ReadinessCheck;
        if let Err(message) = self.wait_until_ready().await {
            return Err(self.fail(message, Vec::new()).await);
        }

        self.state = RunnableState::Started;
        tracing::debug!(
            component = %self.identifier,
            phase = "start_done",
            "started",
        );
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        if self.stopped {
            return Ok(());
        }
        self.state = RunnableState::Stopping;

        tracing::debug!(
            component = %self.identifier,
            phase = "stop_begin",
            "stopping",
        );

        self.stop_container().await;

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }

        tracing::debug!(
            component = %self.identifier,
            phase = "stop_done",
            "stopped",
        );

        let mut causes = Vec::new();
        for child in self.children.iter_mut().flatten().rev() {
            if let Err(fault) = arena::component::stop_child(child).await {
                causes.push(fault);
            }
        }

        self.stopped = true;

        if !causes.is_empty() {
            let fault =
                Fault::component(&self.identifier, message::stop_did_not_complete()).caused_by_all(causes);
            self.faults.push(fault.clone());
            self.state = RunnableState::Faulted;
            return Err(fault);
        }

        self.state = RunnableState::Stopped;
        Ok(())
    }

    fn release(&mut self) {
        self.container_id = None;
        self.stopped = true;
        for child in self.children.iter_mut().flatten().rev() {
            arena::component::release_child(child);
        }
        self.state = RunnableState::Stopped;
    }

    async fn force_stop(&mut self) {
        let removed = self.force_remove_container().await;
        self.stopped = true;

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }

        for child in self.children.iter_mut().flatten().rev() {
            arena::component::force_stop_child(child).await;
        }

        if removed {
            self.state = RunnableState::Stopped;
            return;
        }

        let unconfirmed = Fault::component(
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

    fn add_child(&mut self, child: Box<dyn RunnableComponent>) {
        self.children.get_or_insert_with(Vec::new).push(child);
    }

    fn children(&self) -> &[Component] {
        self.children.as_deref().unwrap_or(&[])
    }

    fn children_mut(&mut self) -> &mut [Component] {
        self.children.as_deref_mut().unwrap_or(&mut [])
    }
}
