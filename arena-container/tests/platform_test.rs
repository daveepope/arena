use arena_container::platform::docker_platform;

#[test]
fn docker_platform_returns_linux_prefixed_value() {
    assert!(docker_platform().starts_with("linux/"));
}

#[test]
fn docker_platform_maps_current_arch_to_docker_naming() {
    let expected_arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    };
    assert_eq!(docker_platform(), format!("linux/{expected_arch}"));
}
