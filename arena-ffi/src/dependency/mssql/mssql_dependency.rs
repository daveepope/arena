use arena::Dependency;
use arena_mssql::{MssqlDependency, MssqlEncryption};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EncryptionConfig {
    #[default]
    Off,
    On,
}

impl From<EncryptionConfig> for MssqlEncryption {
    fn from(value: EncryptionConfig) -> Self {
        match value {
            EncryptionConfig::Off => MssqlEncryption::Off,
            EncryptionConfig::On => MssqlEncryption::On,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct MssqlDependencyConfig {
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
    #[serde(default)]
    pub encryption: Option<EncryptionConfig>,
}

pub(crate) fn build(config: &MssqlDependencyConfig, network: Option<&str>) -> Result<Dependency, String> {
    let mut builder = MssqlDependency::builder(&config.identifier)
        .with_image(config.image.as_deref().unwrap_or("2022-CU14-ubuntu-22.04"));
    if let Some(ref image_name) = config.image_name {
        builder = builder.with_image_name(image_name);
    }
    builder = builder
        .with_port(config.port.unwrap_or(1433))
        .with_database_name(config.database_name.as_deref().unwrap_or("arena_db"))
        .with_database_username(config.database_username.as_deref().unwrap_or("sa"))
        .with_database_password(
            config
                .database_password
                .as_deref()
                .unwrap_or("yourStrong(!)Password"),
        )
        .with_encryption(config.encryption.unwrap_or_default().into())
        .with_startup_sql_scripts(config.startup_sql_scripts.clone().unwrap_or_default());
    if let Some(network) = network {
        builder = builder.with_network(network);
    }
    if let Some(ref container_name) = config.container_name {
        builder = builder.with_container_name(container_name);
    }
    Ok(Box::new(builder.build()))
}

