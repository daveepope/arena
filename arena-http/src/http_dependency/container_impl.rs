use async_trait::async_trait;
use testcontainers_modules::testcontainers;
use testcontainers_modules::testcontainers::core::{ContainerPort, IntoContainerPort, WaitFor};
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{GenericImage, ImageExt};
use crate::http_dependency::HttpImpl;

const DEFAULT_CONTAINER_PORT: u16 = 8080;

pub(crate) struct HttpContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<GenericImage>>,
    base_url: Option<String>,
    network: Option<String>,
}

impl HttpContainerImpl {
    pub(crate) fn new(network: Option<String>) -> Self {
        Self {
            container: None,
            base_url: None,
            network,
        }
    }
}

#[async_trait]
impl HttpImpl for HttpContainerImpl {
    async fn start(&mut self, port: u16, image_name: &str, image_tag: &str, container_name: &str) {
        if self.container.is_some() {
            return;
        }

        arena_container::container::try_remove_existing_container(container_name).await;

        let container_port: ContainerPort = DEFAULT_CONTAINER_PORT.tcp();

        let mut request = GenericImage::new(image_name, image_tag)
            .with_exposed_port(container_port)
            .with_wait_for(WaitFor::message_on_stdout("port:"))
            .with_container_name(container_name);

        if port > 0 {
            request = request.with_mapped_port(port, container_port);
        }

        if let Some(ref network) = self.network {
            arena_container::network::ensure_network_exists(network).await;
            request = request.with_network(network);
        }

        let container = request
            .start()
            .await
            .expect("start http dependency");

        let host = container
            .get_host()
            .await
            .expect("Failed to get host")
            .to_string();

        let port = container
            .get_host_port_ipv4(container_port)
            .await
            .expect("Failed to get port");

        self.base_url = Some(format!("http://{host}:{port}"));
        self.container = Some(container);

        log::info!("[HttpImpl] started dependency.");
    }

    async fn stop(&mut self) {
        self.container.take();
        self.base_url = None;
        log::info!("[HttpImpl] stopped dependency.");

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
    }

    fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    fn admin_url(&self) -> Option<String> {
        self.base_url.as_ref().map(|url| format!("{url}/__admin"))
    }
}
