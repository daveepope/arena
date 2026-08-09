use arena::{Component, Dependency, Match, MatchTrait};
use arena_oauth::{build_oauth_dependency_from_config, OauthFfiDependencyConfig};
use futures::future::{BoxFuture, FutureExt};
use serde::Deserialize;

use crate::containerized_component;
use crate::executable_component;
use crate::dependency::http::http_dependency;
use crate::dependency::localstack::localstack_dependency;
use crate::dependency::mssql::mssql_dependency;
use crate::dependency::smtp::smtp_dependency;
use crate::dependency::temporal::temporal_dependency;
use crate::kafka_dependency;
use crate::managed_playbook;
use crate::postgres_dependency;

const DEFAULT_MATCH_NAME: &str = "arena-match";

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) struct MatchConfig {
    pub network: Option<String>,
    pub match_name: Option<String>,
    pub dependencies: Option<Vec<DependencyNode>>,
    pub components: Option<Vec<ComponentNode>>,
    pub playbooks: Option<Vec<managed_playbook::ManagedPlaybookConfig>>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum DependencyConfig {
    Postgres(postgres_dependency::PostgresDependencyConfig),
    Mssql(mssql_dependency::MssqlDependencyConfig),
    Kafka(kafka_dependency::KafkaDependencyConfig),
    Http(http_dependency::HttpDependencyConfig),
    Localstack(localstack_dependency::LocalstackDependencyConfig),
    Oauth(OauthFfiDependencyConfig),
    Temporal(temporal_dependency::TemporalDependencyConfig),
    Smtp(smtp_dependency::SmtpDependencyConfig),
}

#[derive(Debug, Deserialize)]
pub(crate) struct DependencyNode {
    #[serde(flatten)]
    pub config: DependencyConfig,
    #[serde(default)]
    pub children: Vec<DependencyNode>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ComponentConfig {
    Exec(executable_component::ExecutableComponentConfig),
    #[serde(rename = "container")]
    Containerized(containerized_component::ContainerizedComponentConfig),
}

#[derive(Debug, Deserialize)]
pub(crate) struct ComponentNode {
    #[serde(flatten)]
    pub config: ComponentConfig,
    #[serde(default)]
    pub children: Vec<ComponentNode>,
}

pub(crate) async fn build_match_async(config: &MatchConfig) -> Result<Box<dyn MatchTrait>, String> {
    let network = config.network.as_deref();
    let match_name = config.match_name.as_deref().unwrap_or(DEFAULT_MATCH_NAME);

    let dependencies = build_dependencies(config, network)?;
    let components = build_components_async(config).await?;

    let mut a_match = Match::new(match_name, dependencies, components);
    for playbook_cfg in config.playbooks.as_deref().unwrap_or(&[]) {
        let exec_on_dependency_start = playbook_cfg.exec_on_dependency_start;
        let boxed = managed_playbook::build(playbook_cfg.clone());
        a_match = a_match.register_playbook(boxed, exec_on_dependency_start);
    }
    Ok(Box::new(a_match))
}

fn build_dependencies(config: &MatchConfig, network: Option<&str>) -> Result<Vec<Dependency>, String> {
    config
        .dependencies
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|node| build_dependency_node(node, network))
        .collect()
}

const MAX_CHILDREN_DEPTH: usize = 32;

fn build_dependency_node(node: &DependencyNode, network: Option<&str>) -> Result<Dependency, String> {
    build_dependency_node_at_depth(node, network, 0)
}

fn build_dependency_node_at_depth(
    node: &DependencyNode,
    network: Option<&str>,
    depth: usize,
) -> Result<Dependency, String> {
    if depth >= MAX_CHILDREN_DEPTH {
        return Err(format!(
            "dependency children nesting exceeds max depth of {MAX_CHILDREN_DEPTH}"
        ));
    }
    let mut dependency = build_dependency(&node.config, network)?;
    for child in &node.children {
        dependency.add_child(build_dependency_node_at_depth(child, network, depth + 1)?);
    }
    Ok(dependency)
}

fn build_dependency(config: &DependencyConfig, network: Option<&str>) -> Result<Dependency, String> {
    match config {
        DependencyConfig::Postgres(p) => postgres_dependency::build(p, network),
        DependencyConfig::Mssql(m) => mssql_dependency::build(m, network),
        DependencyConfig::Kafka(k) => kafka_dependency::build(k, network),
        DependencyConfig::Http(h) => http_dependency::build(h, network),
        DependencyConfig::Localstack(l) => localstack_dependency::build(l, network),
        DependencyConfig::Oauth(o) => build_oauth_dependency_from_config(o, network),
        DependencyConfig::Temporal(t) => temporal_dependency::build(t, network),
        DependencyConfig::Smtp(s) => smtp_dependency::build(s, network),
    }
}

async fn build_components_async(config: &MatchConfig) -> Result<Vec<Component>, String> {
    let nodes = config.components.as_deref().unwrap_or(&[]);
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        out.push(build_component_node(node).await?);
    }
    Ok(out)
}

fn build_component_node(node: &ComponentNode) -> BoxFuture<'_, Result<Component, String>> {
    build_component_node_at_depth(node, 0)
}

fn build_component_node_at_depth(
    node: &ComponentNode,
    depth: usize,
) -> BoxFuture<'_, Result<Component, String>> {
    async move {
        if depth >= MAX_CHILDREN_DEPTH {
            return Err(format!(
                "component children nesting exceeds max depth of {MAX_CHILDREN_DEPTH}"
            ));
        }
        let mut component = build_component(&node.config).await?;
        for child in &node.children {
            component.add_child(build_component_node_at_depth(child, depth + 1).await?);
        }
        Ok(component)
    }
    .boxed()
}

async fn build_component(config: &ComponentConfig) -> Result<Component, String> {
    match config {
        ComponentConfig::Exec(e) => executable_component::build(e),
        ComponentConfig::Containerized(ct) => containerized_component::build(ct).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_dependency_variants_config() -> MatchConfig {
        serde_json::from_str(
            r#"{
                "dependencies": [
                    {"type": "postgres", "identifier": "pg"},
                    {"type": "mssql", "identifier": "mssql"},
                    {"type": "kafka", "identifier": "kafka"},
                    {"type": "http", "identifier": "http"},
                    {"type": "localstack", "identifier": "localstack"},
                    {"type": "oauth", "identifier": "oauth"},
                    {"type": "temporal", "identifier": "temporal"},
                    {"type": "smtp", "identifier": "smtp"}
                ]
            }"#,
        )
        .expect("valid match config")
    }

    fn exec_component_config() -> MatchConfig {
        serde_json::from_str(
            r#"{
                "components": [
                    {"type": "exec", "identifier": "exec", "executable_path": "/bin/true"}
                ]
            }"#,
        )
        .expect("valid match config")
    }

    fn full_match_config_with_playbook() -> MatchConfig {
        serde_json::from_str(
            r#"{
                "match_name": "custom-match",
                "network": "arena-net",
                "dependencies": [{"type": "http", "identifier": "http"}],
                "components": [{"type": "exec", "identifier": "exec", "executable_path": "/bin/true"}],
                "playbooks": [{
                    "identifier": "pb",
                    "kind": "http",
                    "dependency_identifier": "http",
                    "mappings": []
                }]
            }"#,
        )
        .expect("valid match config")
    }

    fn nested_dependency_config() -> MatchConfig {
        serde_json::from_str(
            r#"{
                "dependencies": [
                    {
                        "type": "postgres",
                        "identifier": "pg-parent",
                        "children": [
                            {"type": "http", "identifier": "http-child"}
                        ]
                    }
                ]
            }"#,
        )
        .expect("valid nested match config")
    }

    fn nested_component_config() -> MatchConfig {
        serde_json::from_str(
            r#"{
                "components": [
                    {
                        "type": "exec",
                        "identifier": "exec-parent",
                        "executable_path": "/bin/true",
                        "children": [
                            {"type": "exec", "identifier": "exec-child", "executable_path": "/bin/true"}
                        ]
                    }
                ]
            }"#,
        )
        .expect("valid nested match config")
    }

    #[test]
    fn build_dependencies_all_variants_dispatches_to_each_builder() {
        let config = all_dependency_variants_config();
        let dependencies = build_dependencies(&config, None).expect("all variants build");
        assert_eq!(dependencies.len(), 8);
    }

    #[test]
    fn dependency_node_nested_mixed_types_deserializes_into_tree() {
        let node = &nested_dependency_config().dependencies.expect("dependencies present")[0];
        assert!(matches!(node.config, DependencyConfig::Postgres(_)));
        assert_eq!(node.children.len(), 1);
        assert!(matches!(node.children[0].config, DependencyConfig::Http(_)));
    }

    #[test]
    fn build_dependencies_nested_children_returns_only_root_dependencies() {
        let config = nested_dependency_config();
        let dependencies = build_dependencies(&config, None).expect("nested config builds");
        assert_eq!(dependencies.len(), 1);
    }

    fn deeply_nested_dependency_config(depth: usize) -> MatchConfig {
        let mut json = String::from(r#"{"type": "http", "identifier": "leaf"}"#);
        for i in 0..depth {
            json = format!(
                r#"{{"type": "http", "identifier": "d{i}", "children": [{json}]}}"#
            );
        }
        serde_json::from_str(&format!(r#"{{"dependencies": [{json}]}}"#))
            .expect("valid deeply nested match config")
    }

    #[test]
    fn build_dependencies_depth_within_limit_builds_successfully() {
        let config = deeply_nested_dependency_config(MAX_CHILDREN_DEPTH - 1);
        assert!(build_dependencies(&config, None).is_ok());
    }

    #[test]
    fn build_dependencies_depth_exceeds_limit_returns_err() {
        let config = deeply_nested_dependency_config(MAX_CHILDREN_DEPTH + 1);
        match build_dependencies(&config, None) {
            Err(e) => assert!(e.contains("max depth"), "unexpected error: {e}"),
            Ok(_) => panic!("expected depth limit to be enforced"),
        }
    }

    #[test]
    fn build_components_async_nested_children_returns_only_root_components() {
        let config = nested_component_config();
        let components = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(build_components_async(&config))
            .expect("nested component config builds");
        assert_eq!(components.len(), 1);
    }

    #[test]
    fn build_components_async_exec_variant_dispatches_to_builder() {
        let config = exec_component_config();
        let components = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(build_components_async(&config))
            .expect("exec variant builds");
        assert_eq!(components.len(), 1);
    }

    #[test]
    fn build_match_async_full_config_registers_playbook_with_overrides() {
        let config = full_match_config_with_playbook();
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(build_match_async(&config));
        if let Err(e) = result {
            panic!("expected match to build: {e}");
        }
    }
}
