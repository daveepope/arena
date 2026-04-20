use async_trait::async_trait;
use std::time::Duration;
use testcontainers_modules::localstack::LocalStack;
use testcontainers_modules::testcontainers;
use testcontainers_modules::testcontainers::core::{ContainerPort, Healthcheck};
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::ImageExt;

use crate::localstack_dependency::LocalstackImpl;

pub const LOCALSTACK_INTERNAL_DOCKER_PORT: u16 = 4566;

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
            .with_container_name(container_name);

        if !services.is_empty() {
            request = request.with_env_var("SERVICES", services.join(","));
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

        log::info!("[LocalstackImpl] started container.");
    }

    async fn stop(&mut self) {
        self.container.take();
        self.endpoint = None;
        log::info!("[LocalstackImpl] stopped container.");

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
    }

    fn endpoint_url(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }
}
