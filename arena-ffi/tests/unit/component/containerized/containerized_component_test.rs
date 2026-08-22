use arena_ffi::component::containerized::containerized_component::{build, ContainerizedComponentConfig};

fn config_json(json: &str) -> ContainerizedComponentConfig {
    serde_json::from_str(json).expect("deserialize config")
}

#[test]
fn deserialize_no_volume_mappings_defaults_to_none() {
    let config = config_json(
        r#"{
            "identifier": "web",
            "containerfile": "FROM alpine:3.20"
        }"#,
    );

    assert!(config.volume_mappings.is_none());
}

#[test]
fn deserialize_volume_mappings_parses_host_and_container_paths() {
    let config = config_json(
        r#"{
            "identifier": "web",
            "containerfile": "FROM alpine:3.20",
            "volume_mappings": [
                {"host_path": "/host/one", "container_path": "/container/one"},
                {"host_path": "/host/two", "container_path": "/container/two"}
            ]
        }"#,
    );

    let mappings = config.volume_mappings.expect("volume_mappings present");
    assert_eq!(mappings.len(), 2);
    assert_eq!(mappings[0].host_path, "/host/one");
    assert_eq!(mappings[0].container_path, "/container/one");
    assert_eq!(mappings[1].host_path, "/host/two");
    assert_eq!(mappings[1].container_path, "/container/two");
}

#[test]
fn deserialize_image_and_platform_parses_fields() {
    let config = config_json(
        r#"{
            "identifier": "web",
            "image": "myregistry.example.com/web:1.2.3",
            "platform": "linux/arm64"
        }"#,
    );

    assert!(config.containerfile.is_none());
    assert_eq!(
        config.image.as_deref(),
        Some("myregistry.example.com/web:1.2.3")
    );
    assert_eq!(config.platform.as_deref(), Some("linux/arm64"));
}

#[tokio::test]
async fn build_neither_containerfile_nor_image_returns_err() {
    let config = config_json(r#"{"identifier": "web"}"#);

    let result = build(&config).await;

    match result {
        Err(e) => assert!(e.contains("one of containerfile or image is required")),
        Ok(_) => panic!("expected an error"),
    }
}

#[tokio::test]
async fn build_containerfile_and_image_both_set_returns_err() {
    let config = config_json(
        r#"{
            "identifier": "web",
            "containerfile": "FROM alpine:3.20",
            "image": "myregistry.example.com/web:1.2.3"
        }"#,
    );

    let result = build(&config).await;

    match result {
        Err(e) => assert!(e.contains("specify either containerfile or image, not both")),
        Ok(_) => panic!("expected an error"),
    }
}
