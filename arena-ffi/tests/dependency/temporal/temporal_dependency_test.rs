use arena_ffi::dependency::temporal::temporal_dependency::{build, TemporalDependencyConfig};

fn minimal_temporal_config() -> TemporalDependencyConfig {
    TemporalDependencyConfig {
        identifier: "temporal".to_string(),
        image_name: None,
        image: None,
        port: None,
        ui_port: None,
        container_name: None,
    }
}

#[test]
fn build_minimal_config_returns_dependency() {
    assert!(build(&minimal_temporal_config(), None).is_ok());
}

#[test]
fn build_image_and_port_overrides_apply() {
    let mut config = minimal_temporal_config();
    config.image = Some("1.24.2".to_string());
    config.image_name = Some("temporalio/auto-setup".to_string());
    config.port = Some(17233);
    config.ui_port = Some(18233);
    config.container_name = Some("temporal-box".to_string());
    assert!(build(&config, Some("arena-net")).is_ok());
}
