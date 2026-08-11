use arena::Component;
use arena_containerized_component::builder::ContainerizedComponentBuilder;
use arena_containerized_component::containerized_component::ContainerizedComponent;
use serde::Deserialize;
use std::collections::HashMap;

use crate::healthcheck::{HttpReadinessCheck, ReadinessCheckConfig, TcpReadinessCheck};
use crate::runtime_args::RuntimeArgConfig;

#[derive(Debug, Deserialize)]
pub(crate) struct PortMappingConfig {
    pub host_port: u16,
    pub container_port: u16,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum MountConfig {
    Bind {
        source: String,
        container_path: String,
        #[serde(default)]
        read_only: bool,
    },
    Volume {
        source: String,
        container_path: String,
        #[serde(default)]
        read_only: bool,
    },
    Tmpfs {
        container_path: String,
        #[serde(default)]
        size_bytes: Option<i64>,
    },
}

#[derive(Debug, Deserialize)]
pub(crate) struct ContainerizedComponentConfig {
    pub identifier: String,
    #[serde(alias = "dockerfile")]
    pub containerfile: String,
    #[serde(default)]
    pub build_context: Option<String>,
    #[serde(default)]
    pub image_tag: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub env_vars: Option<HashMap<String, String>>,
    #[serde(default)]
    pub runtime_args: Option<Vec<RuntimeArgConfig>>,
    #[serde(default)]
    pub port_mappings: Option<Vec<PortMappingConfig>>,
    #[serde(default)]
    pub readiness_checks: Option<Vec<ReadinessCheckConfig>>,
    #[serde(default)]
    pub host_mappings: Option<Vec<String>>,
    #[serde(default)]
    pub mounts: Option<Vec<MountConfig>>,
}

fn apply_config(
    mut builder: ContainerizedComponentBuilder,
    config: &ContainerizedComponentConfig,
) -> ContainerizedComponentBuilder {
    if let Some(ctx) = &config.build_context {
        builder = builder.with_build_context(ctx);
    }
    if let Some(tag) = &config.image_tag {
        builder = builder.with_image_tag(tag);
    }
    if let Some(n) = &config.network {
        builder = builder.with_network(n);
    }
    if let Some(env_vars) = &config.env_vars {
        for (k, v) in env_vars {
            builder = builder.with_env_var(k, v);
        }
    }
    if let Some(runtime_args) = &config.runtime_args {
        for arg in runtime_args {
            builder = builder.with_runtime_arg(&arg.name, &arg.value);
        }
    }
    if let Some(mappings) = &config.port_mappings {
        for m in mappings {
            builder = builder.with_port_mapping(m.host_port, m.container_port);
        }
    }
    if let Some(hosts) = &config.host_mappings {
        for h in hosts {
            builder = builder.with_host_mapping(h);
        }
    }
    if let Some(mounts) = &config.mounts {
        for m in mounts {
            builder = match m {
                MountConfig::Bind {
                    source,
                    container_path,
                    read_only,
                } => builder.with_bind_mount(source, container_path, *read_only),
                MountConfig::Volume {
                    source,
                    container_path,
                    read_only,
                } => builder.with_volume_mount(source, container_path, *read_only),
                MountConfig::Tmpfs {
                    container_path,
                    size_bytes,
                } => builder.with_tmpfs_mount(container_path, *size_bytes),
            };
        }
    }
    if let Some(checks) = &config.readiness_checks {
        for c in checks {
            builder = match c {
                ReadinessCheckConfig::Http { target, timeout_ms } => {
                    builder.with_readiness_check_timeout(
                        HttpReadinessCheck::new(),
                        target.as_str(),
                        *timeout_ms,
                    )
                }
                ReadinessCheckConfig::Tcp { target, timeout_ms } => {
                    builder.with_readiness_check_timeout(
                        TcpReadinessCheck::new(),
                        target.as_str(),
                        *timeout_ms,
                    )
                }
            };
        }
    }
    builder
}

pub(crate) async fn build(config: &ContainerizedComponentConfig) -> Result<Component, String> {
    let builder = ContainerizedComponent::builder(&config.identifier, &config.containerfile);
    let builder = apply_config(builder, config);
    Ok(Box::new(builder.build().await))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arena_containerized_component::containerized_component::ContainerizedComponentImpl;
    use arena_container::mount::MountSpec;
    use std::path::Path;

    struct NoopContainerImpl;

    #[async_trait::async_trait]
    impl ContainerizedComponentImpl for NoopContainerImpl {
        async fn build_image(
            &self,
            _identifier: &str,
            _containerfile: &str,
            _image_tag: &str,
            _build_context: Option<&Path>,
        ) {
        }

        async fn start_container(
            &self,
            _identifier: &str,
            _image_tag: &str,
            _network: Option<&str>,
            _network_alias: Option<&str>,
            _env_vars: &[(String, String)],
            _runtime_args: &[(String, String)],
            _port_mappings: &[(u16, u16)],
            _host_mappings: &[String],
            _mounts: &[MountSpec],
        ) -> String {
            "noop-container-id".to_string()
        }

        fn follow_logs(&self, _container_id: &str, _identifier: &str) {}

        async fn stop_container(&self, _container_id: &str, _identifier: &str) {}
    }

    fn minimal_config() -> ContainerizedComponentConfig {
        ContainerizedComponentConfig {
            identifier: "web".to_string(),
            containerfile: "FROM alpine".to_string(),
            build_context: None,
            image_tag: None,
            network: None,
            env_vars: None,
            runtime_args: None,
            port_mappings: None,
            readiness_checks: None,
            host_mappings: None,
            mounts: None,
        }
    }

    async fn build_with_noop_impl(config: &ContainerizedComponentConfig) -> Component {
        let builder = ContainerizedComponent::builder(&config.identifier, &config.containerfile);
        let builder = apply_config(builder, config).with_impl(NoopContainerImpl);
        Box::new(builder.build().await)
    }

    #[tokio::test]
    async fn build_minimal_config_returns_component() {
        build_with_noop_impl(&minimal_config()).await;
    }

    #[tokio::test]
    async fn build_all_options_set_applies() {
        let mut config = minimal_config();
        config.build_context = Some(".".to_string());
        config.image_tag = Some("web:test".to_string());
        config.network = Some("arena-net".to_string());
        config.env_vars = Some(HashMap::from([("KEY".to_string(), "value".to_string())]));
        config.runtime_args = Some(vec![RuntimeArgConfig {
            name: "--flag".to_string(),
            value: "on".to_string(),
        }]);
        config.port_mappings = Some(vec![PortMappingConfig {
            host_port: 8080,
            container_port: 80,
        }]);
        config.host_mappings = Some(vec!["db.local:127.0.0.1".to_string()]);
        config.readiness_checks = Some(vec![
            ReadinessCheckConfig::Http {
                target: "http://localhost/health".to_string(),
                timeout_ms: 1_000,
            },
            ReadinessCheckConfig::Tcp {
                target: "localhost:5432".to_string(),
                timeout_ms: 1_000,
            },
        ]);
        config.mounts = Some(vec![
            MountConfig::Bind {
                source: std::env::temp_dir().to_string_lossy().into_owned(),
                container_path: "/mnt/bind".to_string(),
                read_only: true,
            },
            MountConfig::Volume {
                source: "data-volume".to_string(),
                container_path: "/mnt/data".to_string(),
                read_only: false,
            },
            MountConfig::Tmpfs {
                container_path: "/mnt/scratch".to_string(),
                size_bytes: Some(1_048_576),
            },
        ]);

        build_with_noop_impl(&config).await;
    }

    #[tokio::test]
    #[should_panic(expected = "bind mount source path does not exist")]
    async fn build_bind_mount_missing_source_panics() {
        let mut config = minimal_config();
        config.mounts = Some(vec![MountConfig::Bind {
            source: "/arena-ffi-test-nonexistent-path".to_string(),
            container_path: "/mnt/data".to_string(),
            read_only: false,
        }]);

        build_with_noop_impl(&config).await;
    }
}
