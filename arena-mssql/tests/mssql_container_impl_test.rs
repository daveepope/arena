use arena_mssql::{build_ado_connection_string, connect_with_timeout, MssqlEncryption};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn test_password() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    format!("pw-{nanos}")
}

const CLOSED_PORT_RANGE: std::ops::RangeInclusive<u16> = 21300..=21349;

async fn closed_port() -> u16 {
    arena_host::find_available_port::find_available_port(
        CLOSED_PORT_RANGE,
        arena_host::find_available_port::PortSearchStrategy::Random,
    )
    .unwrap_or_else(|| {
        panic!(
            "no available port found in range {}..={}",
            CLOSED_PORT_RANGE.start(), CLOSED_PORT_RANGE.end()
        )
    })
}

#[tokio::test]
async fn connect_with_timeout_closed_port_retries_then_reports_attempt_count() {
    let port = closed_port().await;
    let conn = build_ado_connection_string("127.0.0.1", port, "master", "sa", &test_password(), MssqlEncryption::Off);

    let started_at = Instant::now();
    let result = connect_with_timeout(&conn, Some(Duration::from_millis(200))).await;
    let elapsed = started_at.elapsed();

    let err = result.expect_err("expected connect to a closed port to fail");
    assert!(
        err.contains("failed after 3 attempts"),
        "expected retry count in error, got {err:?}"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "expected retries to be bounded, took {elapsed:?}"
    );
}

#[test]
fn build_ado_connection_string_off_includes_danger_plaintext() {
    let conn = build_ado_connection_string("db.example", 1433, "appdb", "sa", &test_password(), MssqlEncryption::Off);
    assert!(conn.contains("TrustServerCertificate=True;"));
    assert!(conn.contains("encrypt=DANGER_PLAINTEXT;"));
}

#[test]
fn build_ado_connection_string_on_omits_encrypt_clause() {
    let conn = build_ado_connection_string("db.example", 1433, "appdb", "sa", &test_password(), MssqlEncryption::On);
    assert!(conn.contains("TrustServerCertificate=True;"));
    assert!(!conn.to_ascii_lowercase().contains("encrypt="));
}

#[test]
fn build_ado_connection_string_default_off_matches_admin_shape() {
    let conn = build_ado_connection_string(
        "127.0.0.1",
        1433,
        "validationDb",
        "sa",
        "secret",
        MssqlEncryption::Off,
    );
    let admin = build_ado_connection_string(
        "127.0.0.1",
        1433,
        "master",
        "sa",
        "secret",
        MssqlEncryption::Off,
    );
    assert!(conn.contains("encrypt=DANGER_PLAINTEXT;"));
    assert!(admin.contains("encrypt=DANGER_PLAINTEXT;"));
}
