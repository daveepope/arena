use arena::dependency::RunnableDependency;
use arena_mssql::{MssqlDependency, MssqlEncryption};

fn ephemeral_tcp_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral tcp port")
        .local_addr()
        .expect("local_addr")
        .port()
}

async fn started_dependency(
    encryption: Option<MssqlEncryption>,
    label: &str,
) -> MssqlDependency {
    let mut builder = MssqlDependency::builder(label).with_port(ephemeral_tcp_port());
    if let Some(mode) = encryption {
        builder = builder.with_encryption(mode);
    }
    let mut dep = builder.build();
    dep.start().await;
    dep
}

fn assert_contains(label: &str, conn_str: &str, needle: &str) {
    assert!(
        conn_str.contains(needle),
        "[{label}] expected connection string to contain {needle:?}, got: {conn_str}"
    );
}

fn assert_not_contains(label: &str, conn_str: &str, needle: &str) {
    assert!(
        !conn_str.to_ascii_lowercase().contains(needle),
        "[{label}] expected connection string NOT to contain {needle:?}, got: {conn_str}"
    );
}

#[tokio::test]
async fn connection_string_default_appends_danger_plaintext_clause() {
    let mut dep = started_dependency(None, "encryption-default").await;

    let conn = dep
        .connection_string()
        .expect("connection string after start")
        .to_string();
    let admin = dep
        .admin_connection_string()
        .expect("admin connection string after start")
        .to_string();

    assert_contains("default", &conn, "encrypt=DANGER_PLAINTEXT;");
    assert_contains("default", &admin, "encrypt=DANGER_PLAINTEXT;");

    dep.stop().await;

    let mut dep = started_dependency(Some(MssqlEncryption::Off), "encryption-off").await;
    let conn = dep
        .connection_string()
        .expect("connection string after start")
        .to_string();
    assert_contains("off", &conn, "TrustServerCertificate=True;");
    assert_contains("off", &conn, "encrypt=DANGER_PLAINTEXT;");
    dep.stop().await;

    let mut dep = started_dependency(Some(MssqlEncryption::On), "encryption-on").await;
    let conn = dep
        .connection_string()
        .expect("connection string after start")
        .to_string();
    assert_contains("on", &conn, "TrustServerCertificate=True;");
    assert_not_contains("on", &conn, "encrypt=");
    dep.stop().await;
}
