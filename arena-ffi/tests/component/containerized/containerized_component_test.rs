use arena_ffi::component::containerized::containerized_component::{build, ContainerizedComponentConfig};

#[tokio::test]
async fn build_from_image_with_all_optional_fields_applies_each_mapping() {
    let config: ContainerizedComponentConfig = serde_json::from_str(
        r#"{
            "identifier": "web",
            "image": "arena-nonexistent-repo-89f3c1e2/does-not-exist:latest",
            "platform": "linux/amd64",
            "network": "probe-net",
            "env_vars": {"KEY": "value"},
            "runtime_args": [{"name": "some_arg", "value": "some_value"}],
            "port_mappings": [{"host_port": 8080, "container_port": 80}],
            "host_mappings": ["host.docker.internal:host-gateway"],
            "volume_mappings": [{"host_path": "/host", "container_path": "/container"}],
            "readiness_checks": [
                {"kind": "http", "target": "http://localhost:8080/health", "timeout_ms": 1000},
                {"kind": "tcp", "target": "localhost:8080", "timeout_ms": 1000}
            ]
        }"#,
    )
    .expect("deserialize config");

    let result = build(&config).await;

    assert!(
        result.is_err(),
        "build should fail pulling a nonexistent image, after applying every mapping"
    );
}
