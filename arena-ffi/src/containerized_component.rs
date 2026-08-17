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
pub(crate) struct VolumeMappingConfig {
    pub host_path: String,
    pub container_path: String,
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
    pub volume_mappings: Option<Vec<VolumeMappingConfig>>,
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
    if let Some(volumes) = &config.volume_mappings {
        for v in volumes {
            builder = builder.with_volume_mapping(&v.host_path, &v.container_path);
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
    fn deserialize_no_volume_mappings_defaults_to_none() {
        let config: ContainerizedComponentConfig = serde_json::from_str(
            r#"{
                "identifier": "web",
                "containerfile": "FROM alpine:3.20"
            }"#,
        )
        .expect("deserialize config");

        assert!(config.volume_mappings.is_none());
    }

    #[test]
    fn deserialize_volume_mappings_parses_host_and_container_paths() {
        let config: ContainerizedComponentConfig = serde_json::from_str(
            r#"{
                "identifier": "web",
                "containerfile": "FROM alpine:3.20",
                "volume_mappings": [
                    {"host_path": "/host/one", "container_path": "/container/one"},
                    {"host_path": "/host/two", "container_path": "/container/two"}
                ]
            }"#,
        )
        .expect("deserialize config");

        let mappings = config.volume_mappings.expect("volume_mappings present");
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].host_path, "/host/one");
        assert_eq!(mappings[0].container_path, "/container/one");
        assert_eq!(mappings[1].host_path, "/host/two");
        assert_eq!(mappings[1].container_path, "/container/two");
    }
}
