use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use crate::http_dependency::container_impl::HttpContainerImpl;
use crate::http_dependency::{HttpDependency, HttpImpl};

pub struct HttpDependencyBuilder {
    identifier: String,
    http_impl: Option<Box<dyn HttpImpl>>,
    port: Option<u16>,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    image_name: Option<String>,
    image_tag: Option<String>,
    container_name: Option<String>,
    network: Option<String>,
    readiness_check: Option<Box<dyn ReadinessCheck>>,
}

impl HttpDependencyBuilder {
    const DEFAULT_PORT: u16 = 0;
    const DEFAULT_IMAGE_NAME: &'static str = "wiremock/wiremock";
    const DEFAULT_IMAGE_TAG: &'static str = "3.13.0";

    pub(crate) fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            http_impl: None,
            port: None,
            dependencies: None,
            image_name: None,
            image_tag: None,
            container_name: None,
            network: None,
            readiness_check: None,
        }
    }

    pub fn with_impl<W>(mut self, wrapper: W) -> Self
    where
        W: HttpImpl + 'static,
    {
        self.http_impl = Some(Box::new(wrapper));
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn with_child_dependencies(
        mut self,
        dependencies: Vec<Box<dyn RunnableDependency>>,
    ) -> Self {
        self.dependencies = Some(dependencies);
        self
    }

    pub fn with_image_name(mut self, image_name: impl Into<String>) -> Self {
        self.image_name = Some(image_name.into());
        self
    }

    pub fn with_image_tag(mut self, image_tag: impl Into<String>) -> Self {
        self.image_tag = Some(image_tag.into());
        self
    }

    pub fn with_image(self, image_tag: impl Into<String>) -> Self {
        self.with_image_tag(image_tag)
    }

    pub fn with_container_name(mut self, container_name: impl Into<String>) -> Self {
        self.container_name = Some(container_name.into());
        self
    }

    pub fn with_network(mut self, network: impl Into<String>) -> Self {
        self.network = Some(network.into());
        self
    }

    pub fn with_readiness_check<W>(mut self, check: W) -> Self
    where
        W: ReadinessCheck + 'static,
    {
        self.readiness_check = Some(Box::new(check));
        self
    }

    pub fn with_container_tag(self, image_tag: impl Into<String>) -> Self {
        self.with_image_tag(image_tag)
    }

    pub fn build(self) -> HttpDependency {
        let http_impl = self
            .http_impl
            .unwrap_or_else(|| Box::new(HttpContainerImpl::new(self.network)));

        let port = self.port.unwrap_or(Self::DEFAULT_PORT);
        let image_name = self
            .image_name
            .unwrap_or_else(|| Self::DEFAULT_IMAGE_NAME.to_string());
        let image_tag = self
            .image_tag
            .unwrap_or_else(|| Self::DEFAULT_IMAGE_TAG.to_string());

        let mut dep = HttpDependency::new(
            self.identifier,
            http_impl,
            port,
            self.dependencies,
            image_name,
            image_tag,
            self.container_name,
        );

        if let Some(check) = self.readiness_check {
            dep.set_readiness_check(check);
        }

        dep
    }
}
