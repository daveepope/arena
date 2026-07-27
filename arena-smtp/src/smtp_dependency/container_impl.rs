use crate::smtp_dependency::SmtpImpl;
use async_trait::async_trait;
use testcontainers_modules::testcontainers::core::ContainerPort;
use testcontainers_modules::testcontainers::ImageExt;
use testcontainers_modules::{
    testcontainers, testcontainers::runners::AsyncRunner, testcontainers::GenericImage,
};

const SMTP_CONTAINER_PORT: u16 = 1025;
const UI_CONTAINER_PORT: u16 = 8025;

pub(crate) struct SmtpContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<GenericImage>>,
    smtp_address: Option<String>,
    http_api_url: Option<String>,
    network: Option<String>,
}

impl SmtpContainerImpl {
    pub(crate) fn new(network: Option<String>) -> Self {
        Self {
            container: None,
            smtp_address: None,
            http_api_url: None,
            network,
        }
    }
}

#[async_trait]
impl SmtpImpl for SmtpContainerImpl {
    async fn start(
        &mut self,
        smtp_port: u16,
        ui_port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    ) {
        if self.container.is_some() {
            return;
        }

        arena_container::container::try_remove_existing_container(container_name).await;

        let smtp_container_port = ContainerPort::from(SMTP_CONTAINER_PORT);
        let ui_container_port = ContainerPort::from(UI_CONTAINER_PORT);

        let image = GenericImage::new(image_name, image_tag)
            .with_exposed_port(smtp_container_port)
            .with_exposed_port(ui_container_port);

        let mut request = image
            .with_container_name(container_name)
            .with_mapped_port(smtp_port, smtp_container_port)
            .with_mapped_port(ui_port, ui_container_port);

        if let Some(ref network) = self.network {
            arena_container::network::ensure_network_exists(network).await;
            request = request.with_network(network);
        }

        let container = request.start().await.expect("start smtp container");

        let (host, smtp_host_port, ui_host_port) = tokio::join!(
            container.get_host(),
            container.get_host_port_ipv4(smtp_container_port),
            container.get_host_port_ipv4(ui_container_port),
        );
        let host = host.expect("Failed to get host").to_string();
        let smtp_host_port = smtp_host_port.expect("Failed to get smtp port");
        let ui_host_port = ui_host_port.expect("Failed to get ui port");

        self.smtp_address = Some(format!("{host}:{smtp_host_port}"));
        self.http_api_url = Some(format!("http://{host}:{ui_host_port}"));
        self.container = Some(container);

        tracing::debug!(layer = "smtp_container", phase = "container_started");
    }

    async fn stop(&mut self) {
        if self.container.take().is_none() {
            return;
        }
        self.smtp_address = None;
        self.http_api_url = None;
        tracing::debug!(layer = "smtp_container", phase = "container_stopped");

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
    }

    fn smtp_address(&self) -> Option<&str> {
        self.smtp_address.as_deref()
    }

    fn http_api_url(&self) -> Option<&str> {
        self.http_api_url.as_deref()
    }
}
