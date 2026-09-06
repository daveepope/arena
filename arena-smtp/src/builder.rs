use std::time::Duration;
use crate::smtp_dependency::container_impl::SmtpContainerImpl;
use crate::smtp_dependency::{SmtpDependency, SmtpImpl, SmtpTlsMode};
use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;

pub struct SmtpDependencyBuilder {
    identifier: String,
    expiry: Option<Duration>,
    smtp_impl: Option<Box<dyn SmtpImpl>>,
    port: Option<u16>,
    ui_port: Option<u16>,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    image_name: Option<String>,
    image_tag: Option<String>,
    container_name: Option<String>,
    network: Option<String>,
    tls_mode: Option<SmtpTlsMode>,
    readiness_check: Option<Box<dyn ReadinessCheck>>,
}

impl SmtpDependencyBuilder {
    const DEFAULT_PORT: u16 = 1025;
    const DEFAULT_UI_PORT: u16 = 8025;

    pub(crate) fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            expiry: Some(arena_container::expiry::DEFAULT_EXPIRY),
            smtp_impl: None,
            port: None,
            ui_port: None,
            dependencies: None,
            image_name: None,
            image_tag: None,
            container_name: None,
            network: None,
            tls_mode: None,
            readiness_check: None,
        }
    }

    pub fn with_impl<W>(mut self, wrapper: W) -> Self
    where
        W: SmtpImpl + 'static,
    {
        self.smtp_impl = Some(Box::new(wrapper));
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Option::from(port);
        self
    }

    pub fn with_ui_port(mut self, ui_port: u16) -> Self {
        self.ui_port = Option::from(ui_port);
        self
    }

    pub fn with_child_dependencies(
        mut self,
        dependencies: Vec<Box<dyn RunnableDependency>>,
    ) -> Self {
        self.dependencies = Option::from(dependencies);
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

    pub fn with_starttls(mut self) -> Self {
        self.tls_mode = Some(SmtpTlsMode::StartTls);
        self
    }

    pub fn with_implicit_tls(mut self) -> Self {
        self.tls_mode = Some(SmtpTlsMode::Implicit);
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

    pub fn with_expiry(mut self, expiry: Duration) -> Self {
        self.expiry = Some(expiry);
        self
    }

    pub fn without_expiry(mut self) -> Self {
        self.expiry = None;
        self
    }

    pub fn build(self) -> SmtpDependency {
        let mut smtp_impl = self
            .smtp_impl
            .unwrap_or_else(|| Box::new(SmtpContainerImpl::new(self.network)));
        smtp_impl.set_expiry(self.expiry);

        let port = self.port.unwrap_or(Self::DEFAULT_PORT);
        let ui_port = self.ui_port.unwrap_or(Self::DEFAULT_UI_PORT);
        let image_name = self
            .image_name
            .unwrap_or_else(|| arena_container::default_images::SMTP.image.to_string());
        let image_tag = self
            .image_tag
            .unwrap_or_else(|| arena_container::default_images::SMTP.tag.to_string());

        let mut dep = SmtpDependency::new(
            arena_container::identifier::build(crate::MODULE, &self.identifier),
            smtp_impl,
            port,
            ui_port,
            self.dependencies,
            image_name,
            image_tag,
            self.container_name,
            self.tls_mode,
        );

        if let Some(check) = self.readiness_check {
            dep.set_readiness_check(check);
        }

        dep
    }
}
