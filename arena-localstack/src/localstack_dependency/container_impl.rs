use async_trait::async_trait;
use std::time::Duration;
use testcontainers_modules::localstack::LocalStack;
use testcontainers_modules::testcontainers;
use testcontainers_modules::testcontainers::core::{ContainerPort, Healthcheck, Mount};
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::ImageExt;

use crate::localstack_dependency::LocalstackImpl;

pub const LOCALSTACK_INTERNAL_DOCKER_PORT: u16 = 4566;

const CONTAINER_DOCKER_SOCK: &str = "/var/run/docker.sock";

fn host_docker_socket_bind_source() -> Option<String> {
    if let Ok(p) = std::env::var("ARENA_DOCKER_SOCKET_PATH") {
        let p = p.trim().to_string();
        if !p.is_empty() {
            return Some(p);
        }
    }

    if let Ok(dh) = std::env::var("DOCKER_HOST") {
        let dh = dh.trim();
        if let Some(u) = dh.strip_prefix("unix://") {
            let path = u.trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    #[cfg(unix)]
    {
        let default = "/var/run/docker.sock";
        if std::path::Path::new(default).exists() {
            Some(default.to_string())
        } else {
            None
        }
    }

    #[cfg(windows)]
    {
        Some("//var/run/docker.sock".to_string())
    }

    #[cfg(all(not(unix), not(windows)))]
    {
        None
    }
}

fn tcp_healthcheck(port: u16) -> Healthcheck {
    Healthcheck::cmd_shell(format!("bash -lc 'echo > /dev/tcp/127.0.0.1/{port}'"))
        .with_interval(Duration::from_millis(250))
        .with_timeout(Duration::from_secs(1))
        .with_retries(120u32)
}

pub(crate) struct LocalstackContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<LocalStack>>,
    endpoint: Option<String>,
    network: Option<String>,
}

impl LocalstackContainerImpl {
    pub(crate) fn new(network: Option<String>) -> Self {
        Self {
            container: None,
            endpoint: None,
            network,
        }
    }
}

#[async_trait]
impl LocalstackImpl for LocalstackContainerImpl {
    async fn start(
        &mut self,
        port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
        services: &[String],
    ) {
        if self.container.is_some() {
            return;
        }

        arena_container::container::try_remove_existing_container(container_name).await;

        const DEFAULT_CONTAINER_PORT: ContainerPort =
            ContainerPort::Tcp(LOCALSTACK_INTERNAL_DOCKER_PORT);

        let healthcheck = tcp_healthcheck(LOCALSTACK_INTERNAL_DOCKER_PORT);

        let mut request = LocalStack::default()
            .with_name(image_name)
            .with_tag(image_tag)
            .with_mapped_port(port, DEFAULT_CONTAINER_PORT)
            .with_health_check(healthcheck)
            .with_container_name(container_name)
            .with_env_var("LS_LOG", "error")
            .with_env_var("DEBUG", "0")
            .with_env_var("PERSISTENCE", "0");

        if !services.is_empty() {
            request = request.with_env_var("SERVICES", services.join(","));
        }

        if services.iter().any(|s| s == "lambda") {
            let host_sock = host_docker_socket_bind_source().unwrap_or_else(|| {
                panic!(
                    "Localstack with the lambda service needs a bind-mountable Docker socket on the host. \
                     Set ARENA_DOCKER_SOCKET_PATH to that path, or set DOCKER_HOST=unix:///path/to/docker.sock. \
                     On Windows with Docker Desktop (Linux engine), the client normally accepts //var/run/docker.sock; \
                     if yours does not, set ARENA_DOCKER_SOCKET_PATH explicitly."
                );
            });
            request = request.with_mount(Mount::bind_mount(host_sock, CONTAINER_DOCKER_SOCK));
        }

        if let Some(ref network) = self.network {
            arena_container::network::ensure_network_exists(network).await;
            request = request.with_network(network);
        }

        let container = request.start().await.expect("start localstack container");

        let host = container
            .get_host()
            .await
            .expect("Failed to get host")
            .to_string();

        let host_port = container
            .get_host_port_ipv4(DEFAULT_CONTAINER_PORT)
            .await
            .expect("Failed to get port");

        self.endpoint = Some(format!("http://{host}:{host_port}"));
        self.container = Some(container);

        tracing::debug!(layer = "localstack_container", phase = "container_started");
    }

    async fn stop(&mut self) {
        self.container.take();
        self.endpoint = None;
        tracing::debug!(layer = "localstack_container", phase = "container_stopped");

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
    }

    fn endpoint_url(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }
}
