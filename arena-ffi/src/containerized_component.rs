use arena::Component;
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

pub(crate) async fn build(config: &ContainerizedComponentConfig) -> Result<Component, String> {
    let mut builder = ContainerizedComponent::builder(&config.identifier, &config.containerfile);
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
    Ok(Box::new(builder.build().await))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_config_bind_deserializes_with_default_read_only() {
        let mount: MountConfig = serde_json::from_str(
            r#"{"type": "bind", "source": "/host/data", "container_path": "/mnt/data"}"#,
        )
        .expect("bind mount config should deserialize");

        match mount {
            MountConfig::Bind {
                source,
                container_path,
                read_only,
            } => {
                assert_eq!(source, "/host/data");
                assert_eq!(container_path, "/mnt/data");
                assert!(!read_only);
            }
            other => panic!("expected Bind variant, got {:?}", other),
        }
    }

    #[test]
    fn mount_config_volume_deserializes_with_explicit_read_only() {
        let mount: MountConfig = serde_json::from_str(
            r#"{"type": "volume", "source": "my-volume", "container_path": "/mnt/data", "read_only": true}"#,
        )
        .expect("volume mount config should deserialize");

        match mount {
            MountConfig::Volume {
                source,
                container_path,
                read_only,
            } => {
                assert_eq!(source, "my-volume");
                assert_eq!(container_path, "/mnt/data");
                assert!(read_only);
            }
            other => panic!("expected Volume variant, got {:?}", other),
        }
    }

    #[test]
    fn mount_config_tmpfs_deserializes_with_default_size() {
        let mount: MountConfig =
            serde_json::from_str(r#"{"type": "tmpfs", "container_path": "/mnt/scratch"}"#)
                .expect("tmpfs mount config should deserialize");

        match mount {
            MountConfig::Tmpfs {
                container_path,
                size_bytes,
            } => {
                assert_eq!(container_path, "/mnt/scratch");
                assert_eq!(size_bytes, None);
            }
            other => panic!("expected Tmpfs variant, got {:?}", other),
        }
    }

    #[test]
    fn mount_config_tmpfs_deserializes_with_explicit_size() {
        let mount: MountConfig = serde_json::from_str(
            r#"{"type": "tmpfs", "container_path": "/mnt/scratch", "size_bytes": 1048576}"#,
        )
        .expect("tmpfs mount config should deserialize");

        match mount {
            MountConfig::Tmpfs { size_bytes, .. } => assert_eq!(size_bytes, Some(1_048_576)),
            other => panic!("expected Tmpfs variant, got {:?}", other),
        }
    }

    #[test]
    fn containerized_component_config_dockerfile_alias_populates_containerfile() {
        let config: ContainerizedComponentConfig =
            serde_json::from_str(r#"{"identifier": "web", "dockerfile": "FROM alpine"}"#)
                .expect("config should deserialize using the dockerfile alias");

        assert_eq!(config.identifier, "web");
        assert_eq!(config.containerfile, "FROM alpine");
        assert!(config.mounts.is_none());
    }

    #[test]
    fn containerized_component_config_with_mounts_deserializes_all_variants() {
        let config: ContainerizedComponentConfig = serde_json::from_str(
            r#"{
                "identifier": "web",
                "containerfile": "FROM alpine",
                "mounts": [
                    {"type": "bind", "source": "/host", "container_path": "/mnt/bind"},
                    {"type": "volume", "source": "vol", "container_path": "/mnt/vol"},
                    {"type": "tmpfs", "container_path": "/mnt/tmp"}
                ]
            }"#,
        )
        .expect("config with mounts should deserialize");

        let mounts = config.mounts.expect("mounts should be present");
        assert_eq!(mounts.len(), 3);
        assert!(matches!(mounts[0], MountConfig::Bind { .. }));
        assert!(matches!(mounts[1], MountConfig::Volume { .. }));
        assert!(matches!(mounts[2], MountConfig::Tmpfs { .. }));
    }
}
