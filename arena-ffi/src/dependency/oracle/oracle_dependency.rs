use arena::Dependency;
use arena_oracledb::OracleDependency;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OracleDependencyConfig {
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
    pub admin_password: Option<String>,
    #[serde(default)]
    pub container_name: Option<String>,
    #[serde(default)]
    pub startup_sql_scripts: Option<Vec<String>>,
}

pub fn build(config: &OracleDependencyConfig, network: Option<&str>) -> Result<Dependency, String> {
    let mut builder = OracleDependency::builder(&config.identifier);
    if let Some(image) = config.image.as_deref() {
        builder = builder.with_image(image);
    }
    if let Some(ref image_name) = config.image_name {
        builder = builder.with_image_name(image_name);
    }
    if let Some(port) = config.port {
        builder = builder.with_port(port);
    }
    if let Some(ref database_name) = config.database_name {
        builder = builder.with_database_name(database_name);
    }
    if let Some(ref database_username) = config.database_username {
        builder = builder.with_database_username(database_username);
    }
    if let Some(ref database_password) = config.database_password {
        builder = builder.with_database_password(database_password);
    }
    if let Some(ref admin_password) = config.admin_password {
        builder = builder.with_admin_password(admin_password);
    }
    if let Some(ref scripts) = config.startup_sql_scripts {
        builder = builder.with_startup_sql_scripts(scripts.clone());
    }
    if let Some(network) = network {
        builder = builder.with_network(network);
    }
    if let Some(ref container_name) = config.container_name {
        builder = builder.with_container_name(container_name);
    }
    Ok(Box::new(builder.build()))
}
