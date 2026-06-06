use crate::builder::ContainerizedComponentBuilder;
use arena::component::RunnableComponent;
use arena::healthcheck::ReadinessCheck;
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
    pub(crate) children: Option<Vec<Box<dyn RunnableComponent>>>,
    pub(crate) image_tag: String,
    pub(crate) network: Option<String>,
    pub(crate) network_alias: Option<String>,
    pub(crate) env_vars: Vec<(String, String)>,
    pub(crate) runtime_args: Vec<(String, String)>,
    pub(crate) port_mappings: Vec<(u16, u16)>,
    pub(crate) readiness_checks: Vec<(Box<dyn ReadinessCheck>, String, u64)>,
    pub(crate) host_mappings: Vec<String>,
    pub(crate) runtime_client: Docker,
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

    async fn create_and_start_container(&mut self) {
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
            .unwrap_or_else(|e| {
                panic!(
                    "{}: failed to create container: {}",
                    self.identifier, e
                )
            });

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
            .unwrap_or_else(|e| {
                panic!(
                    "{}: failed to start container: {}",
                    self.identifier, e
                )
            });

        tracing::debug!(
            component = %self.identifier,
            container_id_prefix = %id_short,
            phase = "container_running",
            "container started",
        );
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
    async fn start(&mut self) {
        for child in self.children.iter_mut().flatten() {
            child.start().await;
        }

        tracing::debug!(
            component = %self.identifier,
            phase = "start_begin",
            "starting",
        );

        self.create_and_start_container().await;
        self.spawn_log_follower();
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

        self.stop_container().await;

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
