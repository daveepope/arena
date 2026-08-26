use arena_container::identifier::{build, resolve_container_name, sanitize_for_container};

const SUFFIX_LEN: usize = 6;

#[test]
fn builds_module_name_suffix_when_name_given() {
    let id = build("arena-http", "calibration service");
    assert!(id.starts_with("arena-http-calibration-service-"));
    assert!(!id.contains(' '));
    let suffix = id.rsplit('-').next().unwrap();
    assert_eq!(suffix.len(), SUFFIX_LEN);
    assert!(suffix
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
}

#[test]
fn builds_module_suffix_when_name_empty() {
    let id = build("arena-mssql", "");
    assert!(id.starts_with("arena-mssql-"));
    assert!(!id.contains(' '));
}

#[test]
fn treats_whitespace_only_name_as_absent() {
    let id = build("arena-kafka", "   ");
    assert!(id.starts_with("arena-kafka-"));
    assert_eq!(id.matches('-').count(), 2);
}

#[test]
fn two_calls_produce_different_suffixes() {
    let a = build("arena-http", "x");
    let b = build("arena-http", "x");
    assert_ne!(a, b);
}

#[test]
fn builds_module_name_suffix_when_slug_is_a_word_the_size_of_a_suffix() {
    let a = build("arena-oracledb", "example-api-oracle");
    let b = build("arena-oracledb", "example-api-oracle");
    assert!(a.starts_with("arena-oracledb-example-api-oracle-"));
    assert_ne!(
        resolve_container_name(&a, None),
        resolve_container_name(&b, None)
    );
}

#[test]
fn is_idempotent_when_identifier_already_built() {
    let once = build("arena-http", "calibration");
    let twice = build("arena-http", &once);
    assert_eq!(once, twice);
}

#[test]
fn sanitize_is_noop_on_clean_identifier() {
    let id = "arena-mssql-example-validation-a1b2c3";
    assert_eq!(sanitize_for_container(id), id);
}

#[test]
fn sanitize_collapses_spaces_and_non_alphanumerics() {
    assert_eq!(sanitize_for_container("Hello World!!"), "hello-world");
}

#[test]
fn resolve_container_name_uses_override_when_set() {
    assert_eq!(
        resolve_container_name("arena-http-calibration-abc123", Some("custom-name")),
        "custom-name"
    );
}

#[test]
fn resolve_container_name_derives_from_identifier_when_override_missing() {
    assert_eq!(
        resolve_container_name("Hello World!!", None),
        "hello-world"
    );
}
