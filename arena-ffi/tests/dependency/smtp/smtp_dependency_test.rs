use arena_ffi::dependency::smtp::smtp_dependency::{build, SmtpDependencyConfig};

fn minimal_smtp_config() -> SmtpDependencyConfig {
    SmtpDependencyConfig {
        identifier: "smtp".to_string(),
        image_name: None,
        image: None,
        port: None,
        ui_port: None,
        container_name: None,
        starttls: false,
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
    config.starttls = true;
    assert!(build(&config, Some("arena-net")).is_ok());
}
