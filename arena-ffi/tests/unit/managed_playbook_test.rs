use arena_ffi::managed_playbook::{build, ManagedPlaybookConfig};

fn playbook_config(json: &str) -> ManagedPlaybookConfig {
    serde_json::from_str(json).expect("valid managed playbook config")
}

#[test]
fn build_all_kinds_dispatches_to_each_builder() {
    let configs = [
        playbook_config(
            r#"{"identifier": "pb-http", "kind": "http", "dependency_identifier": "http", "mappings": []}"#,
        ),
        playbook_config(r#"{"identifier": "pb-mssql", "kind": "mssql", "dependency_identifier": "mssql"}"#),
        playbook_config(r#"{"identifier": "pb-oracle", "kind": "oracledb", "dependency_identifier": "oracle"}"#),
        playbook_config(
            r#"{"identifier": "pb-localstack", "kind": "localstack", "dependency_identifier": "localstack"}"#,
        ),
        playbook_config(r#"{"identifier": "pb-postgres", "kind": "postgres", "dependency_identifier": "postgres"}"#),
    ];

    let identifiers: Vec<String> = configs.into_iter().map(|config| build(config).identifier().to_string()).collect();

    assert_eq!(identifiers, vec!["pb-http", "pb-mssql", "pb-oracle", "pb-localstack", "pb-postgres"]);
}
