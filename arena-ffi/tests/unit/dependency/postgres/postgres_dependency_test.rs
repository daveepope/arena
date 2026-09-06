use arena_ffi::dependency::postgres::postgres_dependency::{build, PostgresDependencyConfig};

fn minimal_postgres_config() -> PostgresDependencyConfig {
    PostgresDependencyConfig {
        identifier: "pg".to_string(),
        expiry_seconds: None,
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
