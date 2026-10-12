use arena::healthcheck::ReadinessCheck;
use arena_ffi::healthcheck::{HttpReadinessCheck, ReadinessCheckConfig, TcpReadinessCheck};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn respond_once(listener: TcpListener, status_line: &'static str) {
    if let Ok((mut socket, _)) = listener.accept().await {
        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await;
        let response = format!("HTTP/1.1 {status_line}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        let _ = socket.write_all(response.as_bytes()).await;
    }
}

async fn respond_repeatedly(listener: TcpListener, status_line: &'static str) {
    loop {
        let Ok((mut socket, _)) = listener.accept().await else { break };
        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await;
        let response = format!("HTTP/1.1 {status_line}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        let _ = socket.write_all(response.as_bytes()).await;
    }
}

#[test]
fn deserialize_tcp_defaults_timeout() {
    let config: ReadinessCheckConfig =
        serde_json::from_str(r#"{"kind":"tcp","target":"127.0.0.1:2525"}"#).unwrap();
    match config {
        ReadinessCheckConfig::Tcp { target, timeout_ms } => {
            assert_eq!(target, "127.0.0.1:2525");
            assert_eq!(timeout_ms, 10_000);
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
fn deserialize_http_defaults_timeout() {
    let config: ReadinessCheckConfig =
        serde_json::from_str(r#"{"kind":"http","target":"http://127.0.0.1:8080/health"}"#)
            .unwrap();
    match config {
        ReadinessCheckConfig::Http { target, timeout_ms } => {
            assert_eq!(target, "http://127.0.0.1:8080/health");
            assert_eq!(timeout_ms, 10_000);
        }
        other => panic!("expected Http, got {other:?}"),
    }
}

#[test]
fn is_ready_reachable_target_returns_ok() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target = listener.local_addr().unwrap().to_string();
        let result = TcpReadinessCheck::new().is_ready("svc", &target, 200).await;
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
            .is_ready("svc", &target, 0)
            .await
            .unwrap_err();
        assert!(message.contains("svc"));
        assert!(message.contains("timed out"));
    });
}

#[test]
fn is_ready_http_200_response_returns_ok() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let target = format!("http://{addr}/health");
        tokio::spawn(respond_once(listener, "200 OK"));

        let result = HttpReadinessCheck::new().is_ready("svc", &target, 2_000).await;
        assert!(result.is_ok());
    });
}

#[test]
fn is_ready_http_404_response_times_out() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let target = format!("http://{addr}/health");
        tokio::spawn(respond_repeatedly(listener, "404 Not Found"));

        let message = HttpReadinessCheck::new()
            .is_ready("svc", &target, 300)
            .await
            .unwrap_err();
        assert!(message.contains("svc"));
        assert!(message.contains("timed out"));
    });
}
