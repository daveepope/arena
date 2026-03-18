use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub(super) struct EncounterJson {
    pub network: Option<String>,
    pub encounter_name: Option<String>,
    pub dependencies: Option<Vec<DependencyJson>>,
    pub components: Option<Vec<ComponentJson>>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum DependencyJson {
    Postgres(PostgresJson),
    Kafka(KafkaJson),
}

#[derive(Debug, Deserialize)]
pub(super) struct PostgresJson {
    pub identifier: String,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub database_name: Option<String>,
    #[serde(default)]
    pub database_username: Option<String>,
    #[serde(default)]
    pub database_password: Option<String>,
    #[serde(default)]
    pub container_name: Option<String>,
    #[serde(default)]
    pub startup_sql_scripts: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct KafkaJson {
    pub identifier: String,
    #[serde(default)]
    pub flavor: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub container_name: Option<String>,
    #[serde(default)]
    pub topics: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum ComponentJson {
    Exec(ExecJson),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum BuildToolJson {
    Simple(String),
    Custom { command: String, args: Vec<String> },
}

#[derive(Debug, Deserialize)]
pub(super) struct ExecJson {
    pub identifier: String,
    pub executable_path: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub build_tool: Option<BuildToolJson>,
    #[serde(default)]
    pub env_vars: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub runtime_args: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    #[allow(dead_code)]
    pub readiness_check_url: Option<String>,
}
