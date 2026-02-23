use arena::Component;
use arena::healthcheck::ReadinessCheck;
use crate::container_component::ContainerComponent;
use bollard::query_parameters::BuildImageOptionsBuilder;
use bollard::{body_full, Docker};
use futures::StreamExt;
use std::path::PathBuf;

pub struct ContainerComponentBuilder {
    identifier: String,
    children: Option<Vec<Component>>,
    dockerfile: String,
    build_context: Option<PathBuf>,
    image_tag: Option<String>,
    network: Option<String>,
    network_alias: Option<String>,
    env_vars: Vec<(String, String)>,
    runtime_args: Vec<(String, String)>,
    port_mappings: Vec<(u16, u16)>,
    readiness_checks: Vec<(Box<dyn ReadinessCheck>, String)>,
}

impl ContainerComponentBuilder {
    pub(crate) fn new(identifier: impl Into<String>, dockerfile: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            children: None,
            dockerfile: dockerfile.into(),
            build_context: None,
            image_tag: None,
            network: None,
            network_alias: None,
            env_vars: Vec::new(),
            runtime_args: Vec::new(),
            port_mappings: Vec::new(),
            readiness_checks: Vec::new(),
        }
    }

    pub fn with_child_components(mut self, children: Vec<Component>) -> Self {
        self.children = Some(children);
        self
    }

    pub fn with_build_context(mut self, path: impl Into<PathBuf>) -> Self {
        self.build_context = Some(path.into());
        self
    }

    pub fn with_image_tag(mut self, tag: impl Into<String>) -> Self {
        self.image_tag = Some(tag.into());
        self
    }

    pub fn with_network(mut self, network: impl Into<String>) -> Self {
        self.network = Some(network.into());
        self
    }

    pub fn with_network_alias(mut self, alias: impl Into<String>) -> Self {
        self.network_alias = Some(alias.into());
        self
    }

    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }

    pub fn with_runtime_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.runtime_args.push((key.into(), value.into()));
        self
    }

    pub fn with_port_mapping(mut self, host_port: u16, container_port: u16) -> Self {
        self.port_mappings.push((host_port, container_port));
        self
    }

    pub fn with_readiness_check<R>(mut self, check: R, target: impl Into<String>) -> Self
    where
        R: ReadinessCheck + 'static,
    {
        self.readiness_checks.push((Box::new(check), target.into()));
        self
    }

    fn resolve_path(path: PathBuf) -> PathBuf {
        if path.is_absolute() {
            path
        } else {
            let current_dir = std::env::current_dir()
                .expect("get current directory");

            current_dir
                .ancestors()
                .find_map(|ancestor| {
                    let candidate = ancestor.join(&path);
                    if candidate.exists() {
                        Some(candidate)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| current_dir.join(&path))
        }
    }

    fn create_build_context_tar(identifier: &str, dockerfile: &str, build_context: &Option<PathBuf>) -> Vec<u8> {
        let buf = Vec::new();
        let mut tar = tar::Builder::new(buf);

        let dockerfile_bytes = dockerfile.as_bytes();
        let mut header = tar::Header::new_gnu();
        header.set_size(dockerfile_bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, ".arena.Dockerfile", dockerfile_bytes)
            .expect("add Dockerfile to tar");

        if let Some(ref context_path) = build_context {
            tar.append_dir_all(".", context_path)
                .unwrap_or_else(|e| panic!(
                    "[Component-{}] failed to add build context {:?} to tar: {}",
                    identifier, context_path, e
                ));
        }

        tar.into_inner().expect("finalize tar archive")
    }

    async fn build_image(identifier: &str, dockerfile: &str, image_tag: &str, build_context: &Option<PathBuf>, docker: &Docker) {
        log::info!(
            "[Component-{}] building Docker image '{}'",
            identifier, image_tag
        );

        let tar_body = Self::create_build_context_tar(identifier, dockerfile, build_context);

        let options = BuildImageOptionsBuilder::default()
            .dockerfile(".arena.Dockerfile")
            .t(image_tag)
            .rm(true)
            .build();

        let mut stream = docker.build_image(options, None, Some(body_full(tar_body.into())));

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(ref stream_msg) = info.stream {
                        let msg = stream_msg.trim_end();
                        if !msg.is_empty() {
                            log::info!("[Component-{}] {}", identifier, msg);
                        }
                    }
                    if let Some(ref error) = info.error {
                        panic!(
                            "[Component-{}] docker build error: {}",
                            identifier, error
                        );
                    }
                }
                Err(e) => {
                    panic!(
                        "[Component-{}] docker build failed: {}",
                        identifier, e
                    );
                }
            }
        }

        log::info!(
            "[Component-{}] Docker image '{}' built successfully",
            identifier, image_tag
        );
    }

    pub async fn build(self) -> ContainerComponent {
        let build_context = self.build_context.map(Self::resolve_path);

        let image_tag = self.image_tag
            .unwrap_or_else(|| format!("arena-{}", self.identifier));

        let docker = Docker::connect_with_local_defaults()
            .expect("connect to Docker daemon");

        Self::build_image(&self.identifier, &self.dockerfile, &image_tag, &build_context, &docker).await;

        ContainerComponent {
            identifier: self.identifier,
            children: self.children,
            image_tag,
            network: self.network,
            network_alias: self.network_alias,
            env_vars: self.env_vars,
            runtime_args: self.runtime_args,
            port_mappings: self.port_mappings,
            readiness_checks: self.readiness_checks,
            docker,
            container_id: None,
            stopped: false,
        }
    }
}
