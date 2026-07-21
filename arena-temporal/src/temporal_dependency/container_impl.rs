use crate::temporal_dependency::TemporalImpl;
use async_trait::async_trait;
use std::time::Duration;
use testcontainers_modules::testcontainers::core::{ContainerPort, Healthcheck, WaitFor};
use testcontainers_modules::testcontainers::ImageExt;
use testcontainers_modules::{
    testcontainers, testcontainers::runners::AsyncRunner, testcontainers::GenericImage,
};

const TEMPORAL_GRPC_CONTAINER_PORT: u16 = 7233;
const TEMPORAL_UI_CONTAINER_PORT: u16 = 8233;

fn tcp_healthcheck(port: u16) -> Healthcheck {
    Healthcheck::cmd_shell(format!("nc -z 127.0.0.1 {port}"))
        .with_interval(Duration::from_millis(250))
        .with_timeout(Duration::from_secs(1))
        .with_retries(40u32)
}

pub(crate) struct TemporalContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<GenericImage>>,
    grpc_endpoint: Option<String>,
    ui_url: Option<String>,
    network: Option<String>,
}

impl TemporalContainerImpl {
    pub(crate) fn new(network: Option<String>) -> Self {
        Self {
            container: None,
            grpc_endpoint: None,
            ui_url: None,
            network,
        }
    }
}

#[async_trait]
impl TemporalImpl for TemporalContainerImpl {
    async fn start(
        &mut self,
        grpc_port: u16,
        ui_port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    ) {
        if self.container.is_some() {
            return;
        }

        arena_container::container::try_remove_existing_container(container_name).await;

        let grpc_container_port = ContainerPort::from(TEMPORAL_GRPC_CONTAINER_PORT);
        let ui_container_port = ContainerPort::from(TEMPORAL_UI_CONTAINER_PORT);

        let image = GenericImage::new(image_name, image_tag)
            .with_exposed_port(grpc_container_port)
            .with_exposed_port(ui_container_port)
            .with_wait_for(WaitFor::healthcheck());

        let mut request = image
            .with_health_check(tcp_healthcheck(TEMPORAL_GRPC_CONTAINER_PORT))
            .with_container_name(container_name)
            .with_mapped_port(grpc_port, grpc_container_port)
            .with_mapped_port(ui_port, ui_container_port)
            .with_cmd(["server", "start-dev", "--ip", "0.0.0.0"]);

        if let Some(ref network) = self.network {
            arena_container::network::ensure_network_exists(network).await;
            request = request.with_network(network);
        }

        let container = request.start().await.expect("start temporal container");

        let host = container
            .get_host()
            .await
            .expect("Failed to get host")
            .to_string();

        let grpc_host_port = container
            .get_host_port_ipv4(grpc_container_port)
            .await
            .expect("Failed to get grpc port");

        let ui_host_port = container
            .get_host_port_ipv4(ui_container_port)
            .await
            .expect("Failed to get ui port");

        self.grpc_endpoint = Some(format!("{host}:{grpc_host_port}"));
        self.ui_url = Some(format!("http://{host}:{ui_host_port}"));
        self.container = Some(container);

        tracing::debug!(layer = "temporal_container", phase = "container_started");
    }

    async fn stop(&mut self) {
        if self.container.take().is_none() {
            return;
        }
        self.grpc_endpoint = None;
        self.ui_url = None;
        tracing::debug!(layer = "temporal_container", phase = "container_stopped");

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
    }

    fn grpc_endpoint(&self) -> Option<&str> {
        self.grpc_endpoint.as_deref()
    }

    fn ui_url(&self) -> Option<&str> {
        self.ui_url.as_deref()
    }
}
