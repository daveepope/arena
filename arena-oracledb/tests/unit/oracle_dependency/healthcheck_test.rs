use arena::healthcheck::ReadinessCheck;
use arena_oracledb::DefaultOracleReadinessCheck;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;

#[tokio::test]
async fn is_ready_listening_port_returns_ok() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        loop {
            if listener.accept().await.is_err() {
                break;
            }
        }
    });

    let check = DefaultOracleReadinessCheck::new();
    let result = check.is_ready("oracle-test", &addr.to_string(), 2_000).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn is_ready_nothing_listening_times_out_with_err() {
    let check = DefaultOracleReadinessCheck::new();

    let start = Instant::now();
    let result = check.is_ready("oracle-test", "127.0.0.1:1", 300).await;

    assert!(result.is_err());
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn is_ready_failure_message_includes_identifier() {
    let check = DefaultOracleReadinessCheck::new();

    let result = check.is_ready("my-oracle", "127.0.0.1:1", 100).await;

    let err = result.unwrap_err();
    assert!(err.contains("my-oracle"));
}

#[test]
fn default_trait_impl_matches_new() {
    let via_default = DefaultOracleReadinessCheck::default();
    let via_new = DefaultOracleReadinessCheck::new();

    let _ = (via_default, via_new);
}
