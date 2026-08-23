use arena_ffi::runtime_args::RuntimeArgConfig;

#[test]
fn deserialize_valid_json_returns_config() {
    let config: RuntimeArgConfig = serde_json::from_str(r#"{"name": "flag", "value": "on"}"#).expect("valid config");

    assert_eq!(config.name, "flag");
    assert_eq!(config.value, "on");
}

#[test]
fn deserialize_missing_value_field_returns_err() {
    let result: Result<RuntimeArgConfig, _> = serde_json::from_str(r#"{"name": "flag"}"#);

    assert!(result.is_err());
}
