use crate::containerized_component::ContainerizedComponent;
use arena::healthcheck::ReadinessCheck;
use arena::Component;
use arena_container::mount::{MountSpec, MountType};
use bollard::query_parameters::BuildImageOptionsBuilder;
use bollard::{body_full, Docker};
use futures::StreamExt;
use std::path::PathBuf;

pub struct ContainerizedComponentBuilder {
    identifier: String,
    children: Option<Vec<Component>>,
    containerfile: String,
    build_context: Option<PathBuf>,
    image_tag: Option<String>,
    network: Option<String>,
    network_alias: Option<String>,
    env_vars: Vec<(String, String)>,
    runtime_args: Vec<(String, String)>,
    port_mappings: Vec<(u16, u16)>,
    readiness_checks: Vec<(Box<dyn ReadinessCheck>, String, u64)>,
    host_mappings: Vec<String>,
    mounts: Vec<MountSpec>,
}

const DEFAULT_READINESS_TIMEOUT_MS: u64 = 10_000;

impl ContainerizedComponentBuilder {
    pub(crate) fn new(identifier: impl Into<String>, containerfile: impl Into<String>) -> Self {
        Self {
            identifier: arena_container::identifier::build(
                "arena-containerized-component",
                &identifier.into(),
            ),
            children: None,
            containerfile: containerfile.into(),
            build_context: None,
            image_tag: None,
            network: None,
            network_alias: None,
            env_vars: Vec::new(),
            runtime_args: Vec::new(),
            port_mappings: Vec::new(),
            readiness_checks: Vec::new(),
            host_mappings: Vec::new(),
            mounts: Vec::new(),
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

    pub fn with_host_mapping(mut self, host_mapping: impl Into<String>) -> Self {
        self.host_mappings.push(host_mapping.into());
        self
    }

    pub fn with_bind_mount(
        self,
        host_path: impl Into<String>,
        container_path: impl Into<String>,
        read_only: bool,
    ) -> Self {
        self.with_source_mount(MountType::Bind, host_path, container_path, read_only)
    }

    pub fn with_volume_mount(
        self,
        volume_name: impl Into<String>,
        container_path: impl Into<String>,
        read_only: bool,
    ) -> Self {
        self.with_source_mount(MountType::Volume, volume_name, container_path, read_only)
    }

    pub fn with_tmpfs_mount(
        mut self,
        container_path: impl Into<String>,
        size_bytes: Option<i64>,
    ) -> Self {
        self.mounts.push(MountSpec {
            mount_type: MountType::Tmpfs,
            source: None,
            container_path: container_path.into(),
            read_only: false,
            tmpfs_size_bytes: size_bytes,
        });
        self
    }

    fn with_source_mount(
        mut self,
        mount_type: MountType,
        source: impl Into<String>,
        container_path: impl Into<String>,
        read_only: bool,
    ) -> Self {
        self.mounts.push(MountSpec {
            mount_type,
            source: Some(source.into()),
            container_path: container_path.into(),
            read_only,
            tmpfs_size_bytes: None,
        });
        self
    }

    pub fn with_readiness_check<R>(self, check: R, target: impl Into<String>) -> Self
    where
        R: ReadinessCheck + 'static,
    {
        self.with_readiness_check_timeout(check, target, DEFAULT_READINESS_TIMEOUT_MS)
    }

    pub fn with_readiness_check_timeout<R>(
        mut self,
        check: R,
        target: impl Into<String>,
        timeout_ms: u64,
    ) -> Self
    where
        R: ReadinessCheck + 'static,
    {
        self.readiness_checks
            .push((Box::new(check), target.into(), timeout_ms));
        self
    }

    fn resolve_bind_mounts(identifier: &str, mounts: Vec<MountSpec>) -> Vec<MountSpec> {
        mounts
            .into_iter()
            .map(|mut mount| {
                if mount.mount_type == MountType::Bind {
                    let source = mount
                        .source
                        .take()
                        .expect("bind mount source path must be set");
                    let resolved = arena_container::path::resolve(PathBuf::from(&source));
                    if !resolved.exists() {
                        panic!(
                            "{}: bind mount source path does not exist: {}",
                            identifier,
                            resolved.display()
                        );
                    }
                    mount.source = Some(resolved.to_string_lossy().into_owned());
                }
                mount
            })
            .collect()
    }

    async fn build_image(
        identifier: &str,
        containerfile: &str,
        image_tag: &str,
        build_context: &Option<PathBuf>,
        runtime_client: &Docker,
    ) {
        tracing::debug!(
            component = %identifier,
            image = %image_tag,
            phase = "image_build_begin",
            "building container image",
        );

        let tar_body = arena_container::build_context::create_tar(
            identifier,
            containerfile,
            build_context.as_deref(),
        );

        let options = BuildImageOptionsBuilder::default()
            .dockerfile(".arena.Dockerfile")
            .t(image_tag)
            .rm(true)
            .build();

        let mut stream =
            runtime_client.build_image(options, None, Some(body_full(tar_body.into())));

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(ref stream_msg) = info.stream {
                        let msg = stream_msg.trim_end();
                        if !msg.is_empty() {
                            tracing::debug!(
                                component = %identifier,
                                text = %msg,
                                phase = "image_build_stream",
                                "image build output line",
                            );
                        }
                    }
                    if let Some(ref error_detail) = info.error_detail {
                        let message = error_detail.message.as_deref().unwrap_or("");
                        panic!("{}: image build error: {}", identifier, message);
                    }
                }
                Err(e) => {
                    panic!("{}: image build failed: {}", identifier, e);
                }
            }
        }

        tracing::debug!(
            component = %identifier,
            image = %image_tag,
            phase = "image_build_done",
            "container image built",
        );
    }

    pub async fn build(self) -> ContainerizedComponent {
        let mounts = Self::resolve_bind_mounts(&self.identifier, self.mounts);

        let build_context = self.build_context.map(arena_container::path::resolve);

        let image_tag = self.image_tag.unwrap_or_else(|| {
            arena_container::identifier::sanitize_for_container(&self.identifier)
        });

        let runtime_client =
            Docker::connect_with_local_defaults().expect("connect to container runtime");

        Self::build_image(
            &self.identifier,
            &self.containerfile,
            &image_tag,
            &build_context,
            &runtime_client,
        )
        .await;

        ContainerizedComponent {
            identifier: self.identifier,
            children: self.children,
            image_tag,
            network: self.network,
            network_alias: self.network_alias,
            env_vars: self.env_vars,
            runtime_args: self.runtime_args,
            port_mappings: self.port_mappings,
            readiness_checks: self.readiness_checks,
            host_mappings: self.host_mappings,
            mounts,
            runtime_client,
            container_id: None,
            stopped: false,
        }
    }
}
