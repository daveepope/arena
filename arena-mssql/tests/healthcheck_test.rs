use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_mssql::{
    connect_with_timeout, DefaultMssqlReadinessCheck, MssqlDependency, MssqlImpl,
    DEFAULT_CONNECT_TIMEOUT, DEFAULT_PROBE_TIMEOUT,
};
use async_trait::async_trait;
use futures::FutureExt;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;
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

async fn bind_login_then_stall_endpoint() -> (SocketAddr, JoinHandle<()>, Arc<Notify>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let shutdown = Arc::new(Notify::new());
    let shutdown_signal = shutdown.clone();

    let accept_loop = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_signal.notified() => break,
                accept = listener.accept() => {
                    if let Ok((socket, _)) = accept {
                        let _ = tokio::spawn(parrot_prelogin_then_stall(socket));
                    }
                }
            }
        }
    });
    (addr, accept_loop, shutdown)
}

async fn parrot_prelogin_then_stall(mut socket: TcpStream) {
    let mut header = [0u8; 8];
    if socket.read_exact(&mut header).await.is_err() {
        return;
    }
    let total_len = u16::from_be_bytes([header[2], header[3]]) as usize;
    let body_len = total_len.saturating_sub(8);
    let mut body = vec![0u8; body_len];
    let _ = socket.read_exact(&mut body).await;

    let pkt: [u8; 9] = [
        0x04, 0x01, 0x00, 0x09, 0x00, 0x00, 0x01, 0x00, 0xFF,
    ];
    let _ = socket.write_all(&pkt).await;
    let _ = socket.flush().await;

    let mut sink = [0u8; 1024];
    loop {
        if socket.read(&mut sink).await.unwrap_or(0) == 0 {
            return;
        }
    }
}

fn ado_for(addr: SocketAddr) -> String {
    format!(
        "Server=tcp:{host},{port};Database=master;User Id=sa;Password=irrelevant;TrustServerCertificate=True;encrypt=DANGER_PLAINTEXT;",
        host = addr.ip(),
        port = addr.port(),
    )
}

#[tokio::test]
async fn new_uses_default_probe_timeout() {
    let check = DefaultMssqlReadinessCheck::new();
    assert_eq!(check.probe_timeout(), Some(DEFAULT_PROBE_TIMEOUT));
}

#[tokio::test]
async fn with_probe_timeout_overrides_default() {
    let custom = Duration::from_millis(7);
    let check = DefaultMssqlReadinessCheck::new().with_probe_timeout(Some(custom));
    assert_eq!(check.probe_timeout(), Some(custom));
}

#[tokio::test]
async fn default_constant_is_two_seconds() {
    assert_eq!(DEFAULT_PROBE_TIMEOUT, Duration::from_secs(2));
}

#[tokio::test]
async fn is_ready_silent_endpoint_returns_error_bounded_by_outer_budget() {
    let (addr, accept_loop) = bind_silent_endpoint().await;
    let outer_budget = Duration::from_millis(1500);
    let per_attempt = Duration::from_millis(50);

    let check = DefaultMssqlReadinessCheck::new().with_probe_timeout(Some(per_attempt));

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

#[tokio::test]
async fn default_connect_timeout_constant_is_three_seconds() {
    assert_eq!(DEFAULT_CONNECT_TIMEOUT, Duration::from_secs(3));
}

#[tokio::test]
async fn connect_with_timeout_silent_endpoint_returns_error_within_budget() {
    let (addr, accept_loop) = bind_silent_endpoint().await;
    let budget = Duration::from_millis(150);

    let started_at = Instant::now();
    let result = connect_with_timeout(&ado_for(addr), Some(budget)).await;
    let elapsed = started_at.elapsed();

    accept_loop.abort();

    assert!(result.is_err(), "expected timeout error, got {result:?}");
    assert!(
        elapsed < budget * 10,
        "expected bounded timeout across retries (<{:?}), took {elapsed:?}",
        budget * 10
    );
}

#[tokio::test]
async fn connect_with_timeout_none_does_not_wrap_future() {
    let (addr, accept_loop) = bind_silent_endpoint().await;

    let outcome = tokio::time::timeout(
        Duration::from_millis(50),
        connect_with_timeout(&ado_for(addr), None),
    )
    .await;

    accept_loop.abort();

    assert!(
        outcome.is_err(),
        "expected outer test guard to fire when no internal timeout is set, got {outcome:?}"
    );
}

struct SilentEndpointMssqlImpl {
    conn_str: String,
    admin_conn_str: String,
    started: Arc<Mutex<bool>>,
}

#[async_trait]
impl MssqlImpl for SilentEndpointMssqlImpl {
    async fn start(
        &mut self,
        _port: u16,
        _database_name: &str,
        _database_username: &str,
        _database_password: &str,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
    ) -> Result<(), String> {
        *self.started.lock().unwrap() = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        *self.started.lock().unwrap() = false;
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        true
    }
    fn release(&mut self) {}


    fn connection_string(&self) -> Option<&str> {
        Some(&self.conn_str)
    }

    fn admin_connection_string(&self) -> Option<&str> {
        Some(&self.admin_conn_str)
    }
}

struct AlwaysReadyCheck;

#[async_trait]
impl ReadinessCheck for AlwaysReadyCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _connection_string: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn start_silent_post_readiness_endpoint_panics_within_configured_connect_budget() {
    let (addr, accept_loop) = bind_silent_endpoint().await;
    let started = Arc::new(Mutex::new(false));
    let configured_budget = Duration::from_millis(50);

    let fake = SilentEndpointMssqlImpl {
        conn_str: ado_for(addr),
        admin_conn_str: ado_for(addr),
        started: started.clone(),
    };

    let mut mssql = MssqlDependency::builder("silent_endpoint")
        .with_impl(fake)
        .with_readiness_check(AlwaysReadyCheck)
        .with_connect_timeout(configured_budget)
        .build();

    let started_at = Instant::now();
    let outcome = std::panic::AssertUnwindSafe(async {
        mssql.start().await.expect("start should succeed");
    })
    .catch_unwind()
    .await;
    let elapsed = started_at.elapsed();

    accept_loop.abort();

    assert!(
        outcome.is_err(),
        "expected start() to panic when post-readiness connect hangs"
    );
    assert!(
        *started.lock().unwrap(),
        "expected impl.start() to have been called"
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "expected start() to fail well before the default connect budget would; configured={configured_budget:?} elapsed={elapsed:?}"
    );

    if let Err(panic_payload) = outcome {
        let msg = panic_payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic_payload.downcast_ref::<&'static str>().map(|s| s.to_string()))
            .unwrap_or_default();
        assert!(
            msg.contains("mssql connect exceeded 50ms"),
            "expected panic message to mention connect timeout, got {msg:?}"
        );
    }
}

#[tokio::test]
async fn is_ready_endpoint_that_accepts_then_stalls_is_still_bounded() {
    let (addr, accept_loop, shutdown) = bind_login_then_stall_endpoint().await;
    let outer_budget = Duration::from_millis(1500);
    let per_attempt = Duration::from_millis(100);

    let check = DefaultMssqlReadinessCheck::new().with_probe_timeout(Some(per_attempt));

    let started_at = Instant::now();
    let result = check
        .is_ready("test", &ado_for(addr), outer_budget.as_millis() as u64)
        .await;
    let elapsed = started_at.elapsed();

    shutdown.notify_one();
    accept_loop.abort();

    assert!(result.is_err(), "expected timeout error, got {result:?}");
    assert!(
        elapsed < outer_budget * 2,
        "expected readiness bounded by outer budget (outer={outer_budget:?}), took {elapsed:?}"
    );
    assert!(
        elapsed >= per_attempt * 3,
        "expected elapsed >> per_attempt (proving the loop retried instead of hanging on one attempt); per_attempt={per_attempt:?} elapsed={elapsed:?}"
    );
}
