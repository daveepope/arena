use arena::{Component, Dependency, Match, MatchTrait};
use serde::Deserialize;

use crate::container_component;
use crate::executable_component;
use crate::http_dependency;
use crate::kafka_dependency;
use crate::mssql_dependency;
use crate::postgres_dependency;

const DEFAULT_NETWORK: &str = "arena-network";
const DEFAULT_MATCH_NAME: &str = "arena-match";

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) struct MatchConfig {
    pub network: Option<String>,
    pub match_name: Option<String>,
    pub dependencies: Option<Vec<DependencyConfig>>,
    pub components: Option<Vec<ComponentConfig>>,
    pub on_dependency_startup_handlers: Option<Vec<OnDependencyStartupHandlerConfig>>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum DependencyConfig {
    Postgres(postgres_dependency::PostgresDependencyConfig),
    Mssql(mssql_dependency::MssqlDependencyConfig),
    Kafka(kafka_dependency::KafkaDependencyConfig),
    Http(http_dependency::HttpDependencyConfig),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ComponentConfig {
    Exec(executable_component::ExecutableComponentConfig),
    Container(container_component::ContainerComponentConfig),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum OnDependencyStartupHandlerConfig {
    Http(http_dependency::on_dependency_startup::OnDependencyStartupConfig),
}

pub(crate) async fn build_match_async(
    config: &MatchConfig,
) -> Result<Box<dyn MatchTrait>, String> {
    let network = config.network.as_deref().unwrap_or(DEFAULT_NETWORK);
    let match_name = config
        .match_name
        .as_deref()
        .unwrap_or(DEFAULT_MATCH_NAME);

    let dependencies = build_dependencies(config, network)?;
    let components = build_components_async(config).await?;

    let mut a_match = Match::new(match_name, dependencies, components);
    for handler in config
        .on_dependency_startup_handlers
        .as_deref()
        .unwrap_or(&[])
    {
        let boxed = match handler {
            OnDependencyStartupHandlerConfig::Http(cfg) => {
                http_dependency::on_dependency_startup::build(cfg.clone())
            }
        };
        a_match = a_match.with_on_dependency_startup_handler(boxed);
    }
    Ok(Box::new(a_match))
}

fn build_dependencies(
    config: &MatchConfig,
    network: &str,
) -> Result<Vec<Dependency>, String> {
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
        })
        .collect()
}

async fn build_components_async(
    config: &MatchConfig,
) -> Result<Vec<Component>, String> {
    let comps = config.components.as_deref().unwrap_or(&[]);
    let mut out = Vec::with_capacity(comps.len());
    for c in comps {
        let comp: Component = match c {
            ComponentConfig::Exec(e) => executable_component::build(e)?,
            ComponentConfig::Container(ct) => container_component::build(ct).await?,
        };
        out.push(comp);
    }
    Ok(out)
}
