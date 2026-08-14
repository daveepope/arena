use arena_kafka::kafka_dependency::client::connect_client;

const UNREACHABLE_BOOTSTRAP: &str = "127.0.0.1:1";

#[tokio::test]
async fn connect_client_unreachable_bootstrap_returns_err() {
    let result = connect_client(UNREACHABLE_BOOTSTRAP).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn connect_client_invalid_bootstrap_string_returns_err() {
    let result = connect_client("").await;
    assert!(result.is_err());
}
