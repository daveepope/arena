use super::ContainerizedComponentImpl;
use arena_container::mount::MountSpec;
use async_trait::async_trait;
use bollard::body_full;
use bollard::container::LogOutput;
use bollard::models::{ContainerCreateBody, EndpointSettings, HostConfig, NetworkingConfig, PortBinding};
use bollard::query_parameters::{
    BuildImageOptionsBuilder, CreateContainerOptionsBuilder, LogsOptionsBuilder,
    RemoveContainerOptionsBuilder, StopContainerOptionsBuilder,
};
use bollard::Docker;
use futures::StreamExt;
use std::collections::HashMap;
use std::path::Path;

pub(crate) struct BollardContainerImpl {
    docker: Docker,
}

impl BollardContainerImpl {
    pub(crate) fn new() -> Self {
        Self {
            docker: Docker::connect_with_local_defaults().expect("connect to container runtime"),
        }
    }
}

#[async_trait]
impl ContainerizedComponentImpl for BollardContainerImpl {
    async fn build_image(
        &self,
        identifier: &str,
        containerfile: &str,
        image_tag: &str,
        build_context: Option<&Path>,
    ) {
        tracing::debug!(
            component = %identifier,
            image = %image_tag,
            phase = "image_build_begin",
            "building container image",
        );

        let tar_body = arena_container::build_context::create_tar(identifier, containerfile, build_context);

        let options = BuildImageOptionsBuilder::default()
            .dockerfile(".arena.Dockerfile")
            .t(image_tag)
            .rm(true)
            .build();

        let mut stream = self
            .docker
            .build_image(options, None, Some(body_full(tar_body.into())));

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(ref stream_msg) = info.stream {
                        let msg = stream_msg.trim_end();
                        if !msg.is_empty() {
                            tracing::debug!(
                                component = %identifier,
                                text = %msg,
                                phase = "image_build_stream",
                                "image build output line",
                            );
                        }
                    }
                    if let Some(ref error_detail) = info.error_detail {
                        let message = error_detail.message.as_deref().unwrap_or("");
                        panic!("{}: image build error: {}", identifier, message);
                    }
                }
                Err(e) => {
                    panic!("{}: image build failed: {}", identifier, e);
                }
            }
        }

        tracing::debug!(
            component = %identifier,
            image = %image_tag,
            phase = "image_build_done",
            "container image built",
        );
    }

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
    ) -> String {
        arena_container::container::try_remove_existing_container(identifier).await;

        tracing::debug!(
            component = %identifier,
            image = %image_tag,
            phase = "container_create_begin",
            "creating container from image",
        );

        let env: Vec<String> = env_vars.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        let cmd: Vec<String> = runtime_args.iter().map(|(_k, v)| v.clone()).collect();

        let mut host_config = HostConfig {
            ..Default::default()
        };

        let mut exposed_ports: Vec<String> = Vec::new();
        let mut port_bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();

        for (host_port, container_port) in port_mappings {
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

        if !host_mappings.is_empty() {
            host_config.extra_hosts = Some(host_mappings.to_vec());
        }

        if !mounts.is_empty() {
            host_config.mounts = Some(mounts.iter().map(arena_container::mount::to_docker_mount).collect());
        }

        let mut networking_config: Option<NetworkingConfig> = None;

        if let Some(network) = network {
            arena_container::network::ensure_network_exists(network).await;

            host_config.network_mode = Some(network.to_string());

            let endpoint_config = EndpointSettings {
                aliases: network_alias.map(|a| vec![a.to_string()]),
                ..Default::default()
            };

            let mut endpoints = HashMap::new();
            endpoints.insert(network.to_string(), endpoint_config);

            networking_config = Some(NetworkingConfig {
                endpoints_config: Some(endpoints),
            });
        }

        let body = ContainerCreateBody {
            image: Some(image_tag.to_string()),
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
            .name(&arena_container::identifier::sanitize_for_container(identifier))
            .build();

        let response = self
            .docker
            .create_container(Some(options), body)
            .await
            .unwrap_or_else(|e| panic!("{}: failed to create container: {}", identifier, e));

        let container_id = response.id.clone();
        let id_short = container_id[..12.min(container_id.len())].to_string();

        if network.is_some() {
            tracing::debug!(
                component = %identifier,
                network = %network.unwrap_or(""),
                container_id_prefix = %id_short,
                phase = "container_created",
                "container created with network attachment",
            );
        } else {
            tracing::debug!(
                component = %identifier,
                container_id_prefix = %id_short,
                phase = "container_created",
                "container created",
            );
        }

        self.docker
            .start_container(
                &container_id,
                None::<bollard::query_parameters::StartContainerOptions>,
            )
            .await
            .unwrap_or_else(|e| panic!("{}: failed to start container: {}", identifier, e));

        tracing::debug!(
            component = %identifier,
            container_id_prefix = %id_short,
            phase = "container_running",
            "container started",
        );

        container_id
    }

    fn follow_logs(&self, container_id: &str, identifier: &str) {
        let container_id = container_id.to_string();
        let identifier = identifier.to_string();
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
                            arena_container::logging::log_line(&identifier, &line);
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

    async fn stop_container(&self, container_id: &str, identifier: &str) {
        let id_short = &container_id[..12.min(container_id.len())];

        tracing::debug!(
            component = %identifier,
            container_id_prefix = %id_short,
            phase = "container_stop_begin",
            "stopping container",
        );

        let stop_options = StopContainerOptionsBuilder::default().t(10).build();
        if let Err(e) = self
            .docker
            .stop_container(container_id, Some(stop_options))
            .await
        {
            tracing::warn!(
                component = %identifier,
                error = %e,
                phase = "container_stop",
                "stop container returned error",
            );
        }

        let remove_options = RemoveContainerOptionsBuilder::default().force(true).build();
        if let Err(e) = self
            .docker
            .remove_container(container_id, Some(remove_options))
            .await
        {
            tracing::warn!(
                component = %identifier,
                error = %e,
                phase = "container_remove",
                "remove container returned error",
            );
        }

        tracing::debug!(
            component = %identifier,
            phase = "container_removed",
            "container removed",
        );
    }
}
