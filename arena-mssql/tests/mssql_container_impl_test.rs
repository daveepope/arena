use arena_mssql::{build_ado_connection_string, MssqlEncryption};

#[test]
fn build_ado_connection_string_off_includes_danger_plaintext() {
    let conn = build_ado_connection_string("db.example", 1433, "appdb", "sa", "pw", MssqlEncryption::Off);
    assert!(conn.contains("TrustServerCertificate=True;"));
    assert!(conn.contains("encrypt=DANGER_PLAINTEXT;"));
}

#[test]
fn build_ado_connection_string_on_omits_encrypt_clause() {
    let conn = build_ado_connection_string("db.example", 1433, "appdb", "sa", "pw", MssqlEncryption::On);
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
