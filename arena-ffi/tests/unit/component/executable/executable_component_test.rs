use arena_ffi::component::executable::executable_component::{build, BuildToolConfig, ExecutableComponentConfig};
use arena_ffi::healthcheck::ReadinessCheckConfig;
use arena_ffi::runtime_args::RuntimeArgConfig;
use std::collections::HashMap;

fn minimal_config() -> ExecutableComponentConfig {
    ExecutableComponentConfig {
        identifier: "exec".to_string(),
        executable_path: "/bin/true".to_string(),
        source_path: None,
        build_tool: None,
        env_vars: None,
        runtime_args: None,
        readiness_checks: None,
        readiness_check_url: None,
    }
}

#[test]
fn build_minimal_config_returns_component() {
    assert!(build(&minimal_config()).is_ok());
}

#[test]
fn build_with_source_path_returns_component() {
    let mut config = minimal_config();
    config.source_path = Some("/src/app".to_string());
    assert!(build(&config).is_ok());
}

#[test]
fn build_with_env_vars_and_runtime_args_returns_component() {
    let mut config = minimal_config();
    config.env_vars = Some(HashMap::from([("KEY".to_string(), "value".to_string())]));
    config.runtime_args = Some(vec![RuntimeArgConfig {
        name: "arg".to_string(),
        value: "val".to_string(),
    }]);
    assert!(build(&config).is_ok());
}

#[test]
fn build_tool_known_simple_variants_return_component() {
    for tool in ["cargo", "maven", "gradle", "dotnet", "make", "cmake"] {
        let mut config = minimal_config();
        config.build_tool = Some(BuildToolConfig::Simple(tool.to_string()));
        assert!(build(&config).is_ok(), "expected build_tool {tool} to build successfully");
    }
}

#[test]
fn build_tool_custom_variant_returns_component() {
    let mut config = minimal_config();
    config.build_tool = Some(BuildToolConfig::Custom {
        command: "./build.sh".to_string(),
        args: vec!["--release".to_string()],
    });
    assert!(build(&config).is_ok());
}

#[test]
fn build_tool_unknown_simple_variant_returns_err() {
    let mut config = minimal_config();
    config.build_tool = Some(BuildToolConfig::Simple("unknown-tool".to_string()));

    let result = build(&config);

    match result {
        Err(e) => assert!(e.contains("unknown build_tool")),
        Ok(_) => panic!("expected an error"),
    }
}

#[test]
fn build_readiness_check_url_only_derives_http_check() {
    let mut config = minimal_config();
    config.readiness_check_url = Some("http://localhost:8080/health".to_string());
    assert!(build(&config).is_ok());
}

#[test]
fn build_explicit_readiness_checks_http_and_tcp_return_component() {
    let mut config = minimal_config();
    config.readiness_checks = Some(vec![
        ReadinessCheckConfig::Http {
            target: "http://localhost:8080".to_string(),
            timeout_ms: 1000,
        },
        ReadinessCheckConfig::Tcp {
            target: "localhost:9090".to_string(),
            timeout_ms: 1000,
        },
    ]);
    assert!(build(&config).is_ok());
}

#[test]
fn build_readiness_checks_present_ignores_readiness_check_url() {
    let mut config = minimal_config();
    config.readiness_check_url = Some("http://localhost:8080/health".to_string());
    config.readiness_checks = Some(vec![ReadinessCheckConfig::Tcp {
        target: "localhost:9090".to_string(),
        timeout_ms: 500,
    }]);
    assert!(build(&config).is_ok());
}
