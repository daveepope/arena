use arena_ffi::dependency::smtp::smtp_dependency::{build, SmtpDependencyConfig};

fn minimal_smtp_config() -> SmtpDependencyConfig {
    SmtpDependencyConfig {
        identifier: "smtp".to_string(),
        image_name: None,
        image: None,
        port: None,
        ui_port: None,
        container_name: None,
        tls_mode: None,
    }
}

#[test]
fn build_minimal_config_returns_dependency() {
    assert!(build(&minimal_smtp_config(), None).is_ok());
}

#[test]
fn build_image_and_port_overrides_apply() {
    let mut config = minimal_smtp_config();
    config.image = Some("v1.30.5".to_string());
    config.image_name = Some("axllent/mailpit".to_string());
    config.port = Some(11025);
    config.ui_port = Some(18025);
    config.container_name = Some("smtp-box".to_string());
    config.tls_mode = Some("starttls".to_string());
    assert!(build(&config, Some("arena-net")).is_ok());
}

#[test]
fn build_implicit_tls_mode_returns_dependency() {
    let mut config = minimal_smtp_config();
    config.tls_mode = Some("implicit".to_string());
    assert!(build(&config, None).is_ok());
}

#[test]
fn build_unknown_tls_mode_returns_error() {
    let mut config = minimal_smtp_config();
    config.tls_mode = Some("mtls".to_string());
    assert!(build(&config, None).is_err());
}
