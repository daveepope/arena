use arena_ffi::dependency::mssql::mssql_dependency::{build, EncryptionConfig, MssqlDependencyConfig};

fn minimal_mssql_config() -> MssqlDependencyConfig {
    MssqlDependencyConfig {
        identifier: "mssql".to_string(),
        image_name: None,
        image: None,
        port: None,
        database_name: None,
        database_username: None,
        database_password: None,
        container_name: None,
        startup_sql_scripts: None,
        encryption: None,
    }
}

#[test]
fn build_minimal_config_returns_dependency() {
    assert!(build(&minimal_mssql_config(), None).is_ok());
}

#[test]
fn build_image_overrides_apply() {
    let mut config = minimal_mssql_config();
    config.image = Some("2022-CU25-ubuntu-22.04".to_string());
    config.image_name = Some("mcr.microsoft.com/mssql/server".to_string());
    config.encryption = Some(EncryptionConfig::On);
    config.container_name = Some("mssql-box".to_string());
    assert!(build(&config, Some("arena-net")).is_ok());
}
