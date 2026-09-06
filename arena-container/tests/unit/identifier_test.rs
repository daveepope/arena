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

#[test]
fn build_six_character_name_appends_suffix() {
    for name in ["oracle", "broker", "server", "kafka1"] {
        let id = build("arena-oracledb", name);
        assert!(
            id.starts_with(&format!("arena-oracledb-{name}-")),
            "expected {name} to be prefixed and suffixed, got {id}"
        );
        assert_eq!(id.rsplit('-').next().unwrap().len(), SUFFIX_LEN);
    }
}

#[test]
fn build_six_character_name_twice_produces_different_identifiers() {
    let a = build("arena-oracledb", "oracle");
    let b = build("arena-oracledb", "oracle");
    assert_ne!(a, b);
}

#[test]
fn build_identifier_built_by_a_client_module_is_preserved() {
    for client_built in [
        "arena-oracle-example-api-oracle-a1b2c3",
        "arena-container-web-app-9zzzz0",
        "arena-exec-worker-0a1b2c",
    ] {
        assert_eq!(build("arena-oracledb", client_built), client_built);
    }
}

#[test]
fn build_every_module_with_colliding_name_produces_unique_identifiers() {
    const MODULES: [&str; 11] = [
        "arena-http",
        "arena-kafka",
        "arena-localstack",
        "arena-mssql",
        "arena-oauth",
        "arena-oracledb",
        "arena-postgres",
        "arena-smtp",
        "arena-temporal",
        "arena-containerized-component",
        "arena-executable-component",
    ];

    for module in MODULES {
        let first = build(module, "oracle");
        let second = build(module, "oracle");
        assert!(
            first.starts_with(&format!("{module}-oracle-")),
            "expected {module} to prefix and suffix a six character name, got {first}"
        );
        assert_ne!(first, second, "{module} reused an identifier");
        assert_eq!(build(module, &first), first, "{module} is not idempotent");
    }
}
