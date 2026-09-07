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
    container_name: Option<String>,
    expiry: Option<Duration>,
}

impl TemporalContainerImpl {
    pub(crate) fn new(network: Option<String>) -> Self {
        Self {
            container: None,
            grpc_endpoint: None,
            ui_url: None,
            network,
            container_name: None,
            expiry: Some(arena_container::expiry::DEFAULT_EXPIRY),
        }
    }
}

#[async_trait]
impl TemporalImpl for TemporalContainerImpl {
    fn set_expiry(&mut self, expiry: Option<Duration>) {
        self.expiry = expiry;
    }

    async fn start(
        &mut self,
        grpc_port: u16,
        ui_port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    ) -> Result<(), String> {
        if self.container.is_some() {
            return Ok(());
        }

        arena_container::expiry::remove_expired_containers_if_enabled(crate::MODULE, self.expiry)
            .await;

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
            .with_labels(arena_container::expiry::expiry_labels_for(
                crate::MODULE,
                self.expiry,
            ))
            .with_platform(arena_container::platform::resolve_platform(image_name, image_tag).await)
            .with_mapped_port(grpc_port, grpc_container_port)
            .with_mapped_port(ui_port, ui_container_port)
            .with_cmd(["server", "start-dev", "--ip", "0.0.0.0"]);

        if let Some(ref network) = self.network {
            arena_container::network::ensure_network_exists(network).await;
            request = request.with_network(network);
        }

        let container = request.start().await.map_err(|e| {
            arena_container::container::start_failure_message("temporal", &e)
        })?;

        let (host, grpc_host_port, ui_host_port) = tokio::join!(
            container.get_host(),
            container.get_host_port_ipv4(grpc_container_port),
            container.get_host_port_ipv4(ui_container_port),
        );
        let host = host
            .map_err(|e| format!("temporal container host unavailable: {e}"))?
            .to_string();
        let grpc_host_port =
            grpc_host_port.map_err(|e| format!("temporal grpc port unavailable: {e}"))?;
        let ui_host_port = ui_host_port.map_err(|e| format!("temporal ui port unavailable: {e}"))?;

        self.grpc_endpoint = Some(format!("{host}:{grpc_host_port}"));
        self.ui_url = Some(format!("http://{host}:{ui_host_port}"));
        self.container = Some(container);
        self.container_name = Some(container_name.to_string());

        tracing::debug!(layer = "temporal_container", phase = "container_started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        if self.container.take().is_none() {
            return Ok(());
        }
        self.grpc_endpoint = None;
        self.ui_url = None;
        tracing::debug!(layer = "temporal_container", phase = "container_stopped");

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
        Ok(())
    }

    fn release(&mut self) {
        self.container.take();
        self.grpc_endpoint = None;
        self.ui_url = None;
    }

    async fn force_stop(&mut self) -> bool {
        self.release();

        let removed = match self.container_name.as_deref() {
            Some(name) => arena_container::container::force_remove_container(name).await,
            None => true,
        };

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
        removed
    }

    fn grpc_endpoint(&self) -> Option<&str> {
        self.grpc_endpoint.as_deref()
    }

    fn ui_url(&self) -> Option<&str> {
        self.ui_url.as_deref()
    }
}
