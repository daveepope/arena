use crate::http_dependency::HttpImpl;
use async_trait::async_trait;
use testcontainers_modules::testcontainers;
use testcontainers_modules::testcontainers::core::{ContainerPort, IntoContainerPort, WaitFor};
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{GenericImage, ImageExt};

const DEFAULT_CONTAINER_HTTP_PORT: u16 = 8080;

#[derive(Default)]
pub(crate) struct HttpContainerCliConfig {
    pub(crate) cli_args: Vec<String>,
    pub(crate) https_listener_container_port: Option<u16>,
    pub(crate) https_listener_host_port_map: Option<u16>,
    pub(crate) http_disabled: bool,
}

pub(crate) struct HttpContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<GenericImage>>,
    base_url: Option<String>,
    https_base_url: Option<String>,
    network: Option<String>,
    container_cli: HttpContainerCliConfig,
}

impl HttpContainerImpl {
    pub(crate) fn new(network: Option<String>, container_cli: HttpContainerCliConfig) -> Self {
        Self {
            container: None,
            base_url: None,
            https_base_url: None,
            network,
            container_cli,
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

        let container_port: ContainerPort = DEFAULT_CONTAINER_HTTP_PORT.tcp();
        let http_disabled = self.container_cli.http_disabled;

        let mut image = GenericImage::new(image_name, image_tag)
            .with_wait_for(WaitFor::message_on_stdout("port:"));

        if !http_disabled {
            image = image.with_exposed_port(container_port);
        }

        if let Some(https_p) = self.container_cli.https_listener_container_port {
            image = image.with_exposed_port(https_p.tcp());
        }

        let mut request = image
            .with_container_name(container_name)
            .with_platform(arena_container::platform::docker_platform());

        if !http_disabled && port > 0 {
            request = request.with_mapped_port(port, container_port);
        }

        if let Some(https_p) = self.container_cli.https_listener_container_port {
            let https_cp = https_p.tcp();
            if let Some(host_p) = self.container_cli.https_listener_host_port_map {
                request = request.with_mapped_port(host_p, https_cp);
            } else if http_disabled && port > 0 {
                request = request.with_mapped_port(port, https_cp);
            }
        }

        if let Some(ref network) = self.network {
            arena_container::network::ensure_network_exists(network).await;
            request = request.with_network(network);
        }

        if !self.container_cli.cli_args.is_empty() {
            request = request.with_cmd(self.container_cli.cli_args.iter().cloned());
        }

        let container = request.start().await.expect("start http dependency");

        let host = container
            .get_host()
            .await
            .expect("Failed to get host")
            .to_string();

        if !http_disabled {
            let http_mapped = container
                .get_host_port_ipv4(container_port)
                .await
                .expect("Failed to get port");
            self.base_url = Some(format!("http://{host}:{http_mapped}"));
        }

        if let Some(https_p) = self.container_cli.https_listener_container_port {
            let https_mapped = container
                .get_host_port_ipv4(https_p.tcp())
                .await
                .expect("Failed to get https port");
            let https_origin = format!("https://{host}:{https_mapped}");
            self.https_base_url = Some(https_origin.clone());
            if http_disabled {
                self.base_url = Some(https_origin);
            }
        }

        self.container = Some(container);

        tracing::debug!(layer = "http_stub_container", phase = "dependency_started");
    }

    async fn stop(&mut self) {
        self.container.take();
        self.base_url = None;
        self.https_base_url = None;
        tracing::debug!(layer = "http_stub_container", phase = "dependency_stopped");

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
    }

    fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    fn admin_url(&self) -> Option<String> {
        let root = if self.container_cli.http_disabled {
            self.https_base_url.as_ref().or(self.base_url.as_ref())
        } else {
            self.base_url.as_ref()
        }?;
        Some(format!("{root}/__admin"))
    }

    fn https_base_url(&self) -> Option<&str> {
        self.https_base_url.as_deref()
    }
}
