use arena::{Component, Dependency, Match, MatchTrait};
use arena_oauth::{build_oauth_dependency_from_config, OauthFfiDependencyConfig};
use futures::future::{BoxFuture, FutureExt};
use serde::Deserialize;

use crate::component::containerized::containerized_component;
use crate::component::executable::executable_component;
use crate::dependency::http::http_dependency;
use crate::dependency::kafka::kafka_dependency;
use crate::dependency::localstack::localstack_dependency;
use crate::dependency::mssql::mssql_dependency;
use crate::dependency::oracle::oracle_dependency;
use crate::dependency::postgres::postgres_dependency;
use crate::dependency::smtp::smtp_dependency;
use crate::dependency::temporal::temporal_dependency;
use crate::managed_playbook;

const DEFAULT_MATCH_NAME: &str = "arena-match";

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct MatchConfig {
    pub network: Option<String>,
    pub match_name: Option<String>,
    pub dependencies: Option<Vec<DependencyNode>>,
    pub components: Option<Vec<ComponentNode>>,
    pub playbooks: Option<Vec<managed_playbook::ManagedPlaybookConfig>>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(private_interfaces)]
pub enum DependencyConfig {
    Postgres(postgres_dependency::PostgresDependencyConfig),
    Mssql(mssql_dependency::MssqlDependencyConfig),
    #[serde(rename = "oracledb")]
    Oracle(oracle_dependency::OracleDependencyConfig),
    Kafka(kafka_dependency::KafkaDependencyConfig),
    Http(http_dependency::HttpDependencyConfig),
    Localstack(localstack_dependency::LocalstackDependencyConfig),
    Oauth(OauthFfiDependencyConfig),
    Temporal(temporal_dependency::TemporalDependencyConfig),
    Smtp(smtp_dependency::SmtpDependencyConfig),
}

#[derive(Debug, Deserialize)]
pub struct DependencyNode {
    #[serde(flatten)]
    pub config: DependencyConfig,
    #[serde(default)]
    pub children: Vec<DependencyNode>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(private_interfaces)]
pub enum ComponentConfig {
    Exec(executable_component::ExecutableComponentConfig),
    #[serde(rename = "container")]
    Containerized(containerized_component::ContainerizedComponentConfig),
}

#[derive(Debug, Deserialize)]
pub struct ComponentNode {
    #[serde(flatten)]
    pub config: ComponentConfig,
    #[serde(default)]
    pub children: Vec<ComponentNode>,
}

pub async fn build_match_async(config: &MatchConfig) -> Result<Box<dyn MatchTrait>, String> {
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

pub fn build_dependencies(config: &MatchConfig, network: Option<&str>) -> Result<Vec<Dependency>, String> {
    config
        .dependencies
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|node| build_dependency_node(node, network))
        .collect()
}

pub const MAX_CHILDREN_DEPTH: usize = 32;

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
        DependencyConfig::Oracle(o) => oracle_dependency::build(o, network),
        DependencyConfig::Kafka(k) => kafka_dependency::build(k, network),
        DependencyConfig::Http(h) => http_dependency::build(h, network),
        DependencyConfig::Localstack(l) => localstack_dependency::build(l, network),
        DependencyConfig::Oauth(o) => build_oauth_dependency_from_config(o, network),
        DependencyConfig::Temporal(t) => temporal_dependency::build(t, network),
        DependencyConfig::Smtp(s) => smtp_dependency::build(s, network),
    }
}

pub async fn build_components_async(config: &MatchConfig) -> Result<Vec<Component>, String> {
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
