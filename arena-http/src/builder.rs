use crate::http_dependency::container_impl::{HttpContainerCliConfig, HttpContainerImpl};
use crate::http_dependency::{HttpDependency, HttpImpl};
use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;

const DEFAULT_CONTAINER_HTTP_PORT: u16 = 8080;

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
    https: HttpsListenerSettings,
    trusted_certificate_pem: Option<String>,
}

#[derive(Default)]
struct HttpsListenerSettings {
    listener_port: Option<u16>,
    host_port: Option<u16>,
    keystore_path: Option<String>,
    keystore_password: Option<String>,
    key_password: Option<String>,
    keystore_type: Option<String>,
    http_disabled: bool,
}

pub struct HttpDependencyHttpsBuilder {
    parent: HttpDependencyBuilder,
}

impl HttpDependencyHttpsBuilder {
    pub fn listener_container_port(mut self, port: u16) -> Self {
        self.parent.https.listener_port = Some(port);
        self
    }

    pub fn host_port(mut self, port: u16) -> Self {
        self.parent.https.host_port = Some(port);
        self
    }

    pub fn keystore_path(mut self, path_in_container: impl Into<String>) -> Self {
        self.parent.https.keystore_path = Some(path_in_container.into());
        self
    }

    pub fn keystore_password(mut self, password: impl Into<String>) -> Self {
        self.parent.https.keystore_password = Some(password.into());
        self
    }

    pub fn key_password(mut self, password: impl Into<String>) -> Self {
        self.parent.https.key_password = Some(password.into());
        self
    }

    pub fn keystore_type(mut self, store_type: impl Into<String>) -> Self {
        self.parent.https.keystore_type = Some(store_type.into());
        self
    }

    pub fn http_listener_disabled(mut self, disabled: bool) -> Self {
        self.parent.https.http_disabled = disabled;
        self
    }

    pub fn done(self) -> HttpDependencyBuilder {
        self.parent
    }
}

impl HttpDependencyBuilder {
    const DEFAULT_PORT: u16 = 0;
    const DEFAULT_IMAGE_NAME: &'static str = arena_container::default_images::HTTP.image;
    const DEFAULT_IMAGE_TAG: &'static str = arena_container::default_images::HTTP.tag;

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
            https: HttpsListenerSettings::default(),
            trusted_certificate_pem: None,
        }
    }

    pub fn https(self) -> HttpDependencyHttpsBuilder {
        HttpDependencyHttpsBuilder { parent: self }
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

    pub fn with_trusted_certificate_pem(mut self, pem: impl Into<String>) -> Self {
        self.trusted_certificate_pem = Some(pem.into());
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
        let https = self.https;
        let container_cli_cfg = match &self.http_impl {
            None => build_http_container_cli_config(&self.identifier, https),
            Some(_) => HttpContainerCliConfig::default(),
        };

        let http_impl = self
            .http_impl
            .unwrap_or_else(|| Box::new(HttpContainerImpl::new(self.network, container_cli_cfg)));

        let port = self.port.unwrap_or(Self::DEFAULT_PORT);
        let image_name = self
            .image_name
            .unwrap_or_else(|| Self::DEFAULT_IMAGE_NAME.to_string());
        let image_tag = self
            .image_tag
            .unwrap_or_else(|| Self::DEFAULT_IMAGE_TAG.to_string());

        let mut dep = HttpDependency::new(
            arena_container::identifier::build("arena-http", &self.identifier),
            http_impl,
            port,
            self.dependencies,
            image_name,
            image_tag,
            self.container_name,
            self.trusted_certificate_pem,
        );

        if let Some(check) = self.readiness_check {
            dep.set_readiness_check(check);
        }

        dep
    }
}

fn build_http_container_cli_config(
    identifier: &str,
    s: HttpsListenerSettings,
) -> HttpContainerCliConfig {
    let https_listener_port = s.listener_port;
    let https_host_port = s.host_port;
    let keystore_path = s.keystore_path;
    let keystore_password = s.keystore_password;
    let key_password = s.key_password;
    let keystore_type = s.keystore_type;
    let http_disabled = s.http_disabled;

    if http_disabled && https_listener_port.is_none() {
        panic!(
            "[Http-{identifier}] https().http_listener_disabled(true) requires https().listener_container_port(...)"
        );
    }

    if keystore_path.is_none()
        && (keystore_password.is_some() || key_password.is_some() || keystore_type.is_some())
    {
        panic!(
            "[Http-{identifier}] keystore password / key password / keystore type require https().keystore_path(...)"
        );
    }

    if keystore_path.is_some() && https_listener_port.is_none() {
        panic!(
            "[Http-{identifier}] https().keystore_path(...) requires https().listener_container_port(...)"
        );
    }

    let needs_https_cli = https_listener_port.is_some()
        || keystore_path.is_some()
        || keystore_password.is_some()
        || key_password.is_some()
        || keystore_type.is_some()
        || http_disabled;

    let mut cli_args = Vec::new();

    if needs_https_cli {
        cli_args.push("--port".into());
        cli_args.push(DEFAULT_CONTAINER_HTTP_PORT.to_string());
    }

    if let Some(p) = https_listener_port {
        cli_args.push("--https-port".into());
        cli_args.push(p.to_string());
    }

    if let Some(path) = keystore_path {
        cli_args.push("--https-keystore".into());
        cli_args.push(path);
    }

    if let Some(p) = keystore_password {
        cli_args.push("--keystore-password".into());
        cli_args.push(p);
    }

    if let Some(p) = key_password {
        cli_args.push("--key-manager-password".into());
        cli_args.push(p);
    }

    if let Some(t) = keystore_type {
        cli_args.push("--keystore-type".into());
        cli_args.push(t);
    }

    if http_disabled {
        cli_args.push("--disable-http".into());
    }

    let https_listener_host_port_map = https_host_port.and_then(|p| (p > 0).then_some(p));

    HttpContainerCliConfig {
        cli_args,
        https_listener_container_port: https_listener_port,
        https_listener_host_port_map,
        http_disabled,
    }
}
