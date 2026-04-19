use async_trait::async_trait;
use arena::component::RunnableComponent;
use arena::healthcheck::ReadinessCheck;
use crate::builder::ContainerComponentBuilder;
use bollard::container::LogOutput;
use bollard::models::{ContainerCreateBody, EndpointSettings, HostConfig, NetworkingConfig, PortBinding};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, LogsOptionsBuilder,
    RemoveContainerOptionsBuilder, StopContainerOptionsBuilder,
};
use bollard::Docker;
use futures::StreamExt;
use std::collections::HashMap;

pub struct ContainerComponent {
    pub(crate) identifier: String,
    pub(crate) children: Option<Vec<Box<dyn RunnableComponent>>>,
    pub(crate) image_tag: String,
    pub(crate) network: Option<String>,
    pub(crate) network_alias: Option<String>,
    pub(crate) env_vars: Vec<(String, String)>,
    pub(crate) runtime_args: Vec<(String, String)>,
    pub(crate) port_mappings: Vec<(u16, u16)>,
    pub(crate) readiness_checks: Vec<(Box<dyn ReadinessCheck>, String)>,
    pub(crate) docker: Docker,
    pub(crate) container_id: Option<String>,
    pub(crate) stopped: bool,
}

impl ContainerComponent {
    pub fn builder(identifier: impl Into<String>, dockerfile: impl Into<String>) -> ContainerComponentBuilder {
        ContainerComponentBuilder::new(identifier, dockerfile)
    }

    async fn create_and_start_container(&mut self) {
        arena_container::container::try_remove_existing_container(&self.identifier).await;

        log::info!(
            "[Component-{}] creating container from image '{}'",
            self.identifier, self.image_tag
        );

        let env: Vec<String> = self.env_vars
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let cmd: Vec<String> = self.runtime_args
            .iter()
            .map(|(_k, v)| v.clone())
            .collect();

        let mut host_config = HostConfig {
            ..Default::default()
        };

        let mut exposed_ports: HashMap<String, HashMap<(), ()>> = HashMap::new();
        let mut port_bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();

        for (host_port, container_port) in &self.port_mappings {
            let port_key = format!("{}/tcp", container_port);
            exposed_ports.insert(port_key.clone(), HashMap::new());

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

        // If a network is specified, create the container directly on that
        // network so Docker's embedded DNS (127.0.0.11) is configured from
        // the start.  Using connect_network *after* creation leaves the
        // container's /etc/resolv.conf pointing at the default bridge DNS
        // which cannot resolve container names.
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
            exposed_ports: if exposed_ports.is_empty() { None } else { Some(exposed_ports) },
            host_config: Some(host_config),
            networking_config,
            ..Default::default()
        };

        let options = CreateContainerOptionsBuilder::default()
            .name(&arena_container::identifier::sanitize_for_container(
                &self.identifier,
            ))
            .build();

        let response = self.docker
            .create_container(Some(options), body)
            .await
            .unwrap_or_else(|e| panic!(
                "[Component-{}] failed to create container: {}",
                self.identifier, e
            ));

        self.container_id = Some(response.id.clone());

        if self.network.is_some() {
            log::info!(
                "[Component-{}] container created on network '{}' (id: {})",
                self.identifier,
                self.network.as_deref().unwrap_or(""),
                &response.id[..12.min(response.id.len())]
            );
        } else {
            log::info!(
                "[Component-{}] container created (id: {})",
                self.identifier, &response.id[..12.min(response.id.len())]
            );
        }

        self.docker
            .start_container(&response.id, None::<bollard::query_parameters::StartContainerOptions>)
            .await
            .unwrap_or_else(|e| panic!(
                "[Component-{}] failed to start container: {}",
                self.identifier, e
            ));

        log::info!(
            "[Component-{}] container started (id: {})",
            self.identifier, &response.id[..12.min(response.id.len())]
        );
    }

    fn spawn_log_follower(&self) {
        let container_id = match &self.container_id {
            Some(id) => id.clone(),
            None => return,
        };
        let identifier = self.identifier.clone();
        let docker = self.docker.clone();

        tokio::spawn(async move {
            let options = LogsOptionsBuilder::default()
                .follow(true)
                .stdout(true)
                .stderr(true)
                .build();

            let mut stream = docker.logs(&container_id, Some(options));

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
                        log::warn!(
                            "[Component-{}] error reading container logs: {}",
                            identifier, e
                        );
                        break;
                    }
                }
            }
        });
    }

    fn log_line(identifier: &str, line: &str) {
        if line.contains(" ERROR ") {
            log::error!("[{}] {}", identifier, line);
        } else if line.contains(" WARN ") {
            log::warn!("[{}] {}", identifier, line);
        } else if line.contains(" DEBUG ") {
            log::debug!("[{}] {}", identifier, line);
        } else if line.contains(" TRACE ") {
            log::trace!("[{}] {}", identifier, line);
        } else {
            log::info!("[{}] {}", identifier, line);
        }
    }

    async fn wait_until_ready(&self) {
        if self.readiness_checks.is_empty() {
            return;
        }

        let timeout_ms = 10_000;
        for (check, target) in &self.readiness_checks {
            match check.is_ready(&self.identifier, target, timeout_ms).await {
                Ok(()) => {
                    log::debug!(
                        "[Component-{}] readiness check passed for target: {}",
                        self.identifier, target
                    );
                }
                Err(msg) => {
                    panic!(
                        "[Component-{}] readiness check failed for target {}: {}",
                        self.identifier, target, msg
                    );
                }
            }
        }
        log::debug!("[Component-{}] all readiness checks passed.", self.identifier);
    }

    async fn stop_container(&self) {
        let container_id = match &self.container_id {
            Some(id) => id,
            None => return,
        };

        log::info!(
            "[Component-{}] stopping container (id: {})",
            self.identifier, &container_id[..12.min(container_id.len())]
        );

        let stop_options = StopContainerOptionsBuilder::default()
            .t(10)
            .build();
        if let Err(e) = self.docker.stop_container(container_id, Some(stop_options)).await {
            log::warn!(
                "[Component-{}] error stopping container: {}",
                self.identifier, e
            );
        }

        let remove_options = RemoveContainerOptionsBuilder::default()
            .force(true)
            .build();
        if let Err(e) = self.docker.remove_container(container_id, Some(remove_options)).await {
            log::warn!(
                "[Component-{}] error removing container: {}",
                self.identifier, e
            );
        }

        log::info!("[Component-{}] container removed", self.identifier);
    }
}

#[async_trait]
impl RunnableComponent for ContainerComponent {
    async fn start(&mut self) {
        for child in self.children.iter_mut().flatten() {
            child.start().await;
        }

        log::info!("[Component-{}] starting.", self.identifier);

        self.create_and_start_container().await;
        self.spawn_log_follower();
        self.wait_until_ready().await;

        log::info!("[Component-{}] started.", self.identifier);
    }

    async fn stop(&mut self) {
        if self.stopped {
            return;
        }

        log::info!("[Component-{}] stopping.", self.identifier);

        self.stop_container().await;

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }

        log::info!("[Component-{}] stopped.", self.identifier);

        for child in self.children.iter_mut().flatten().rev() {
            child.stop().await;
        }

        self.stopped = true;
    }

    fn add_child(&mut self, child: Box<dyn RunnableComponent>) {
        self.children.get_or_insert_with(Vec::new).push(child);
    }
}
