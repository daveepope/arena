use arena::Component;
use arena_executable_component::builder::ExecutableComponentBuilder;
use arena_executable_component::executable_component::ExecutableComponent;
use arena_executable_component::BuildTool;
use serde::Deserialize;
use std::collections::HashMap;

use crate::healthcheck::{HttpReadinessCheck, ReadinessCheckConfig};
use crate::runtime_args::RuntimeArgConfig;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum BuildToolConfig {
    Simple(String),
    Custom { command: String, args: Vec<String> },
}

#[derive(Debug, Deserialize)]
pub(crate) struct ExecutableComponentConfig {
    pub identifier: String,
    pub executable_path: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub build_tool: Option<BuildToolConfig>,
    #[serde(default)]
    pub env_vars: Option<HashMap<String, String>>,
    #[serde(default)]
    pub runtime_args: Option<Vec<RuntimeArgConfig>>,
    #[serde(default)]
    pub readiness_checks: Option<Vec<ReadinessCheckConfig>>,
    #[serde(default)]
    pub readiness_check_url: Option<String>,
}

pub(crate) fn build(config: &ExecutableComponentConfig) -> Result<Component, String> {
    let mut builder = ExecutableComponent::builder(&config.identifier)
        .with_executable_path(&config.executable_path);

    if let Some(source_path) = &config.source_path {
        builder = builder.with_source_path(source_path);
    }
    if let Some(build_tool) = &config.build_tool {
        builder = builder.with_build_tool(build_tool_from_config(build_tool)?);
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
    builder = apply_readiness_checks(builder, &readiness_checks_for(config));

    Ok(Box::new(builder.build()))
}

fn readiness_checks_for(config: &ExecutableComponentConfig) -> Vec<ReadinessCheckConfig> {
    let mut v = config.readiness_checks.clone().unwrap_or_default();
    if v.is_empty() {
        if let Some(ref url) = config.readiness_check_url {
            v.push(ReadinessCheckConfig::Http {
                target: url.clone(),
            });
        }
    }
    v
}

fn apply_readiness_checks(
    mut builder: ExecutableComponentBuilder,
    checks: &[ReadinessCheckConfig],
) -> ExecutableComponentBuilder {
    for c in checks {
        builder = match c {
            ReadinessCheckConfig::Http { target } => {
                builder.with_readiness_check(HttpReadinessCheck::new(), target.as_str())
            }
        };
    }
    builder
}

fn build_tool_from_config(spec: &BuildToolConfig) -> Result<BuildTool, String> {
    match spec {
        BuildToolConfig::Simple(s) => match s.as_str() {
            "cargo" => Ok(BuildTool::Cargo),
            "maven" => Ok(BuildTool::Maven),
            "gradle" => Ok(BuildTool::Gradle),
            "dotnet" => Ok(BuildTool::Dotnet),
            "make" => Ok(BuildTool::Make),
            "cmake" => Ok(BuildTool::CMake),
            other => Err(format!("unknown build_tool '{other}'")),
        },
        BuildToolConfig::Custom { command, args } => Ok(BuildTool::Custom {
            command: command.clone(),
            args: args.clone(),
        }),
    }
}
