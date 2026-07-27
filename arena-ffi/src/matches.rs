use arena::{Component, Dependency, Match, MatchTrait};
use arena_oauth::{build_oauth_dependency_from_config, OauthFfiDependencyConfig};
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
    pub dependencies: Option<Vec<DependencyConfig>>,
    pub components: Option<Vec<ComponentConfig>>,
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
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ComponentConfig {
    Exec(executable_component::ExecutableComponentConfig),
    #[serde(rename = "container")]
    Containerized(containerized_component::ContainerizedComponentConfig),
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
        .map(|d| match d {
            DependencyConfig::Postgres(p) => postgres_dependency::build(p, network),
            DependencyConfig::Mssql(m) => mssql_dependency::build(m, network),
            DependencyConfig::Kafka(k) => kafka_dependency::build(k, network),
            DependencyConfig::Http(h) => http_dependency::build(h, network),
            DependencyConfig::Localstack(l) => localstack_dependency::build(l, network),
            DependencyConfig::Oauth(o) => build_oauth_dependency_from_config(o, network),
            DependencyConfig::Temporal(t) => temporal_dependency::build(t, network),
            DependencyConfig::Smtp(s) => smtp_dependency::build(s, network),
        })
        .collect()
}

async fn build_components_async(config: &MatchConfig) -> Result<Vec<Component>, String> {
    let comps = config.components.as_deref().unwrap_or(&[]);
    let mut out = Vec::with_capacity(comps.len());
    for c in comps {
        let comp: Component = match c {
            ComponentConfig::Exec(e) => executable_component::build(e)?,
            ComponentConfig::Containerized(ct) => containerized_component::build(ct).await?,
        };
        out.push(comp);
    }
    Ok(out)
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

    #[test]
    fn build_dependencies_all_variants_dispatches_to_each_builder() {
        let config = all_dependency_variants_config();
        let dependencies = build_dependencies(&config, None).expect("all variants build");
        assert_eq!(dependencies.len(), 8);
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
