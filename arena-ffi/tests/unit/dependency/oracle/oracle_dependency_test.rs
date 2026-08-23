use arena_ffi::dependency::oracle::oracle_dependency::{build, OracleDependencyConfig};
use std::time::{SystemTime, UNIX_EPOCH};

fn test_password() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    format!("pw-{nanos}")
}

fn minimal_oracle_config() -> OracleDependencyConfig {
    OracleDependencyConfig {
        identifier: "oracle".to_string(),
        image_name: None,
        image: None,
        port: None,
        database_name: None,
        database_username: None,
        database_password: None,
        admin_password: None,
        container_name: None,
        startup_sql_scripts: None,
        setup_mode: None,
        sql_readiness_timeout_ms: None,
    }
}

#[test]
fn build_minimal_config_returns_dependency() {
    assert!(build(&minimal_oracle_config(), None).is_ok());
}

#[test]
fn build_image_and_credential_overrides_apply() {
    let mut config = minimal_oracle_config();
    config.image = Some("23.26.2-slim-faststart".to_string());
    config.image_name = Some("gvenzl/oracle-free".to_string());
    config.port = Some(15210);
    config.database_name = Some("CUSTOMPDB".to_string());
    config.database_username = Some("custom_user".to_string());
    config.database_password = Some(test_password());
    config.admin_password = Some(test_password());
    config.container_name = Some("oracle-box".to_string());
    config.startup_sql_scripts = Some(vec!["CREATE TABLE widgets (id NUMBER);".to_string()]);
    config.setup_mode = Some("full_build".to_string());
    config.sql_readiness_timeout_ms = Some(120_000);
    assert!(build(&config, Some("arena-net")).is_ok());
}

#[test]
fn build_custom_database_name_without_full_build_setup_mode_panics() {
    let mut config = minimal_oracle_config();
    config.database_name = Some("CUSTOMPDB".to_string());
    let result = std::panic::catch_unwind(|| build(&config, None));
    assert!(result.is_err());
}

#[test]
fn build_unknown_setup_mode_returns_err() {
    let mut config = minimal_oracle_config();
    config.setup_mode = Some("bogus".to_string());
    assert!(build(&config, None).is_err());
}
