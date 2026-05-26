use arena::healthcheck::ReadinessCheck;
use arena_mssql::{DefaultMssqlReadinessCheck, DEFAULT_CONNECT_TIMEOUT};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

async fn bind_silent_endpoint() -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let accept_loop = tokio::spawn(async move {
        let mut held = Vec::new();
        loop {
            match listener.accept().await {
                Ok((socket, _)) => held.push(socket),
                Err(_) => break,
            }
        }
    });
    (addr, accept_loop)
}

fn ado_for(addr: SocketAddr) -> String {
    format!(
        "Server=tcp:{host},{port};Database=master;User Id=sa;Password=irrelevant;TrustServerCertificate=True;",
        host = addr.ip(),
        port = addr.port(),
    )
}

#[tokio::test]
async fn new_uses_default_connect_timeout() {
    let check = DefaultMssqlReadinessCheck::new();
    assert_eq!(check.connect_timeout(), Some(DEFAULT_CONNECT_TIMEOUT));
}

#[tokio::test]
async fn with_connect_timeout_overrides_default() {
    let custom = Duration::from_millis(7);
    let check = DefaultMssqlReadinessCheck::new().with_connect_timeout(Some(custom));
    assert_eq!(check.connect_timeout(), Some(custom));
}

#[tokio::test]
async fn default_constant_is_two_seconds() {
    assert_eq!(DEFAULT_CONNECT_TIMEOUT, Duration::from_secs(2));
}

#[tokio::test]
async fn is_ready_silent_endpoint_returns_error_bounded_by_outer_budget() {
    let (addr, accept_loop) = bind_silent_endpoint().await;
    let outer_budget = Duration::from_millis(1500);
    let per_attempt = Duration::from_millis(50);

    let check = DefaultMssqlReadinessCheck::new().with_connect_timeout(Some(per_attempt));

    let started_at = Instant::now();
    let result = check
        .is_ready("test", &ado_for(addr), outer_budget.as_millis() as u64)
        .await;
    let elapsed = started_at.elapsed();

    accept_loop.abort();

    assert!(result.is_err(), "expected timeout error, got {result:?}");
    assert!(
        elapsed < outer_budget * 2,
        "expected readiness bounded by outer budget (outer={outer_budget:?}), took {elapsed:?}"
    );
    assert!(
        elapsed >= per_attempt * 6,
        "expected elapsed >> per_attempt (proving the loop retried instead of hanging on one attempt); per_attempt={per_attempt:?} elapsed={elapsed:?}"
    );
}
