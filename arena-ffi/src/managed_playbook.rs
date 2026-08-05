use arena::Playbook;
use arena_http::{ManagedHttpPlaybook, Playbook as HttpPlaybook};
use arena_localstack::ManagedLocalstackPlaybook;
use arena_mssql::ManagedMssqlPlaybook;
use arena_postgres::ManagedPostgresPlaybook;
use serde::Deserialize;

use crate::dependency::http::mapping::{build_playbook_from_mappings, MappingSpec};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ManagedPlaybookConfig {
    pub identifier: String,
    #[serde(default = "default_exec_on_dependency_start")]
    pub exec_on_dependency_start: bool,
    #[serde(flatten)]
    pub kind: PlaybookKindConfig,
}

fn default_exec_on_dependency_start() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum PlaybookKindConfig {
    Http(HttpPlaybookConfig),
    Mssql(MssqlPlaybookConfig),
    Localstack(LocalstackPlaybookConfig),
    Postgres(PostgresPlaybookConfig),
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct HttpPlaybookConfig {
    pub dependency_identifier: String,
    pub mappings: Vec<MappingSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MssqlPlaybookConfig {
    pub dependency_identifier: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct LocalstackPlaybookConfig {
    pub dependency_identifier: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PostgresPlaybookConfig {
    pub dependency_identifier: String,
}

pub(crate) fn build(config: ManagedPlaybookConfig) -> Box<dyn Playbook> {
    match config.kind {
        PlaybookKindConfig::Http(http) => Box::new(ManagedHttpPlaybook::new(
            config.identifier,
            http.dependency_identifier,
            move |pb| build_http_playbook(pb, &http.mappings),
        )),
        PlaybookKindConfig::Mssql(mssql) => Box::new(ManagedMssqlPlaybook::new(
            config.identifier,
            mssql.dependency_identifier,
        )),
        PlaybookKindConfig::Localstack(localstack) => Box::new(ManagedLocalstackPlaybook::new(
            config.identifier,
            localstack.dependency_identifier,
        )),
        PlaybookKindConfig::Postgres(postgres) => Box::new(ManagedPostgresPlaybook::new(
            config.identifier,
            postgres.dependency_identifier,
        )),
    }
}

fn build_http_playbook(pb: HttpPlaybook, mappings: &[MappingSpec]) -> HttpPlaybook {
    build_playbook_from_mappings(pb, mappings)
        .unwrap_or_else(|e| panic!("http playbook registration failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playbook_config(json: &str) -> ManagedPlaybookConfig {
        serde_json::from_str(json).expect("valid managed playbook config")
    }

    #[test]
    fn build_all_kinds_dispatches_to_each_builder() {
        let configs = [
            playbook_config(
                r#"{"identifier": "pb-http", "kind": "http", "dependency_identifier": "http", "mappings": []}"#,
            ),
            playbook_config(
                r#"{"identifier": "pb-mssql", "kind": "mssql", "dependency_identifier": "mssql"}"#,
            ),
            playbook_config(
                r#"{"identifier": "pb-localstack", "kind": "localstack", "dependency_identifier": "localstack"}"#,
            ),
            playbook_config(
                r#"{"identifier": "pb-postgres", "kind": "postgres", "dependency_identifier": "postgres"}"#,
            ),
        ];

        let identifiers: Vec<String> = configs
            .into_iter()
            .map(|config| build(config).identifier().to_string())
            .collect();

        assert_eq!(
            identifiers,
            vec!["pb-http", "pb-mssql", "pb-localstack", "pb-postgres"]
        );
    }
}
