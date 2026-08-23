use arena::Component;
use arena_containerized_component::containerized_component::ContainerizedComponent;
use serde::Deserialize;
use std::collections::HashMap;

use crate::healthcheck::{HttpReadinessCheck, ReadinessCheckConfig, TcpReadinessCheck};
use crate::runtime_args::RuntimeArgConfig;

#[derive(Debug, Deserialize)]
pub struct PortMappingConfig {
    pub host_port: u16,
    pub container_port: u16,
}

#[derive(Debug, Deserialize)]
pub struct VolumeMappingConfig {
    pub host_path: String,
    pub container_path: String,
}

#[derive(Debug, Deserialize)]
pub struct ContainerizedComponentConfig {
    pub identifier: String,
    #[serde(alias = "dockerfile", default)]
    pub containerfile: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub build_context: Option<String>,
    #[serde(default)]
    pub image_tag: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
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

pub async fn build(config: &ContainerizedComponentConfig) -> Result<Component, String> {
    let mut builder = match (&config.containerfile, &config.image) {
        (Some(containerfile), None) => {
            ContainerizedComponent::builder(&config.identifier, containerfile)
        }
        (None, Some(image)) => ContainerizedComponent::from_image(&config.identifier, image),
        (Some(_), Some(_)) => {
            return Err(format!(
                "{}: specify either containerfile or image, not both",
                config.identifier
            ))
        }
        (None, None) => {
            return Err(format!(
                "{}: one of containerfile or image is required",
                config.identifier
            ))
        }
    };
    if let Some(platform) = &config.platform {
        builder = builder.with_platform(platform);
    }
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
    builder
        .build()
        .await
        .map(|component| Box::new(component) as Component)
        .map_err(|e| e.to_string())
}
