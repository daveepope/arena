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
    pub image_name: Option<String>,
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
    pub image_name: Option<String>,
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
    Container(ContainerJson),
}

#[derive(Debug, Deserialize)]
pub(super) struct PortMappingJson {
    pub host_port: u16,
    pub container_port: u16,
}

/// Tagged union for `readiness_checks` in exec/container JSON. Keep in sync with
/// [`crate::readiness_json`] dispatch and client serializers (e.g. arena-pytest `_ffi_readiness`).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum ReadinessCheckJson {
    Http { target: String },
}

#[derive(Debug, Deserialize)]
pub(super) struct ContainerJson {
    pub identifier: String,
    pub dockerfile: String,
    #[serde(default)]
    pub build_context: Option<String>,
    #[serde(default)]
    pub image_tag: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub env_vars: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub runtime_args: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub port_mappings: Option<Vec<PortMappingJson>>,
    #[serde(default)]
    pub readiness_checks: Option<Vec<ReadinessCheckJson>>,
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
    pub readiness_checks: Option<Vec<ReadinessCheckJson>>,
    /// Legacy single URL; merged into readiness when `readiness_checks` is empty.
    #[serde(default)]
    pub readiness_check_url: Option<String>,
}
