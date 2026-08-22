use arena_ffi::component::containerized::containerized_component::{build, ContainerizedComponentConfig};

fn config_json(json: &str) -> ContainerizedComponentConfig {
    serde_json::from_str(json).expect("deserialize config")
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
