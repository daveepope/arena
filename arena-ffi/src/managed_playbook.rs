use arena::Playbook;
use arena_http::{ManagedHttpPlaybook, Playbook as HttpPlaybook};
use arena_localstack::ManagedLocalstackPlaybook;
use arena_mssql::ManagedMssqlPlaybook;
use arena_oracledb::ManagedOraclePlaybook;
use arena_postgres::ManagedPostgresPlaybook;
use serde::Deserialize;

use crate::dependency::http::mapping::{build_playbook_from_mappings, MappingSpec};

#[derive(Debug, Clone, Deserialize)]
pub struct ManagedPlaybookConfig {
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
#[allow(private_interfaces)]
pub enum PlaybookKindConfig {
    Http(HttpPlaybookConfig),
    Mssql(MssqlPlaybookConfig),
    Oracle(OraclePlaybookConfig),
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
pub(crate) struct OraclePlaybookConfig {
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

pub fn build(config: ManagedPlaybookConfig) -> Box<dyn Playbook> {
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
        PlaybookKindConfig::Oracle(oracle) => Box::new(ManagedOraclePlaybook::new(
            config.identifier,
            oracle.dependency_identifier,
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
