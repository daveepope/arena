use super::*;

#[test]
fn deserialize_tcp_defaults_timeout() {
    let config: ReadinessCheckConfig =
        serde_json::from_str(r#"{"kind":"tcp","target":"127.0.0.1:2525"}"#).unwrap();
    match config {
        ReadinessCheckConfig::Tcp { target, timeout_ms } => {
            assert_eq!(target, "127.0.0.1:2525");
            assert_eq!(timeout_ms, default_readiness_timeout_ms());
        }
        other => panic!("expected Tcp, got {other:?}"),
    }
}

#[test]
fn deserialize_tcp_reads_explicit_timeout() {
    let config: ReadinessCheckConfig =
        serde_json::from_str(r#"{"kind":"tcp","target":"db:5432","timeout_ms":250}"#).unwrap();
    match config {
        ReadinessCheckConfig::Tcp { target, timeout_ms } => {
            assert_eq!(target, "db:5432");
            assert_eq!(timeout_ms, 250);
        }
        other => panic!("expected Tcp, got {other:?}"),
    }
}

#[test]
fn is_ready_reachable_target_returns_ok() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target = listener.local_addr().unwrap().to_string();
        let result = TcpReadinessCheck::new().is_ready("svc", &target, 1_000).await;
        assert!(result.is_ok());
    });
}

#[test]
fn is_ready_unreachable_target_times_out() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target = listener.local_addr().unwrap().to_string();
        drop(listener);
        let message = TcpReadinessCheck::new()
            .is_ready("svc", &target, 250)
            .await
            .unwrap_err();
        assert!(message.contains("svc"));
        assert!(message.contains("timed out"));
    });
}
