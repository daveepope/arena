use arena::Dependency;
use arena_postgres::PostgresDependency;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct PostgresDependencyConfig {
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

pub(crate) fn build(
    config: &PostgresDependencyConfig,
    network: Option<&str>,
) -> Result<Dependency, String> {
    let mut builder = PostgresDependency::builder(&config.identifier);
    if let Some(image) = config.image.as_deref() {
        builder = builder.with_image(image);
    }
    if let Some(ref image_name) = config.image_name {
        builder = builder.with_image_name(image_name);
    }
    builder = builder
        .with_port(config.port.unwrap_or(5432))
        .with_database_name(config.database_name.as_deref().unwrap_or("arena_db"))
        .with_database_username(config.database_username.as_deref().unwrap_or("arena_user"))
        .with_database_password(config.database_password.as_deref().unwrap_or("postgres"))
        .with_startup_sql_scripts(config.startup_sql_scripts.clone().unwrap_or_default());
    if let Some(network) = network {
        builder = builder.with_network(network);
    }
    if let Some(ref container_name) = config.container_name {
        builder = builder.with_container_name(container_name);
    }
    Ok(Box::new(builder.build()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_postgres_config() -> PostgresDependencyConfig {
        PostgresDependencyConfig {
            identifier: "pg".to_string(),
            image_name: None,
            image: None,
            port: None,
            database_name: None,
            database_username: None,
            database_password: None,
            container_name: None,
            startup_sql_scripts: None,
        }
    }

    #[test]
    fn build_minimal_config_returns_dependency() {
        assert!(build(&minimal_postgres_config(), None).is_ok());
    }

    #[test]
    fn build_image_overrides_apply() {
        let mut config = minimal_postgres_config();
        config.image = Some("17".to_string());
        config.image_name = Some("postgres".to_string());
        config.container_name = Some("pg-box".to_string());
        assert!(build(&config, Some("arena-net")).is_ok());
    }
}
