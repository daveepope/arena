use arena_container::identifier::{build, sanitize_for_container};

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
