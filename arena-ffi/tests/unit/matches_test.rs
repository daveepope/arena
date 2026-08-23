use arena_ffi::matches::{
    build_components_async, build_dependencies, build_match_async, DependencyConfig, MatchConfig,
    MAX_CHILDREN_DEPTH,
};

fn all_dependency_variants_config() -> MatchConfig {
    serde_json::from_str(
        r#"{
            "dependencies": [
                {"type": "postgres", "identifier": "pg"},
                {"type": "mssql", "identifier": "mssql"},
                {"type": "oracledb", "identifier": "oracle"},
                {"type": "kafka", "identifier": "kafka"},
                {"type": "http", "identifier": "http"},
                {"type": "localstack", "identifier": "localstack"},
                {"type": "oauth", "identifier": "oauth"},
                {"type": "temporal", "identifier": "temporal"},
                {"type": "smtp", "identifier": "smtp"}
            ]
        }"#,
    )
    .expect("valid match config")
}

fn exec_component_config() -> MatchConfig {
    serde_json::from_str(
        r#"{
            "components": [
                {"type": "exec", "identifier": "exec", "executable_path": "/bin/true"}
            ]
        }"#,
    )
    .expect("valid match config")
}

fn full_match_config_with_playbook() -> MatchConfig {
    serde_json::from_str(
        r#"{
            "match_name": "custom-match",
            "network": "arena-net",
            "dependencies": [{"type": "http", "identifier": "http"}],
            "components": [{"type": "exec", "identifier": "exec", "executable_path": "/bin/true"}],
            "playbooks": [{
                "identifier": "pb",
                "kind": "http",
                "dependency_identifier": "http",
                "mappings": []
            }]
        }"#,
    )
    .expect("valid match config")
}

fn nested_dependency_config() -> MatchConfig {
    serde_json::from_str(
        r#"{
            "dependencies": [
                {
                    "type": "postgres",
                    "identifier": "pg-parent",
                    "children": [
                        {"type": "http", "identifier": "http-child"}
                    ]
                }
            ]
        }"#,
    )
    .expect("valid nested match config")
}

fn nested_component_config() -> MatchConfig {
    serde_json::from_str(
        r#"{
            "components": [
                {
                    "type": "exec",
                    "identifier": "exec-parent",
                    "executable_path": "/bin/true",
                    "children": [
                        {"type": "exec", "identifier": "exec-child", "executable_path": "/bin/true"}
                    ]
                }
            ]
        }"#,
    )
    .expect("valid nested match config")
}

#[test]
fn build_dependencies_all_variants_dispatches_to_each_builder() {
    let config = all_dependency_variants_config();
    let dependencies = build_dependencies(&config, None).expect("all variants build");
    assert_eq!(dependencies.len(), 9);
}

#[test]
fn dependency_node_nested_mixed_types_deserializes_into_tree() {
    let node = &nested_dependency_config().dependencies.expect("dependencies present")[0];
    assert!(matches!(node.config, DependencyConfig::Postgres(_)));
    assert_eq!(node.children.len(), 1);
    assert!(matches!(node.children[0].config, DependencyConfig::Http(_)));
}

#[test]
fn build_dependencies_nested_children_returns_only_root_dependencies() {
    let config = nested_dependency_config();
    let dependencies = build_dependencies(&config, None).expect("nested config builds");
    assert_eq!(dependencies.len(), 1);
}

fn deeply_nested_dependency_config(depth: usize) -> MatchConfig {
    let mut json = String::from(r#"{"type": "http", "identifier": "leaf"}"#);
    for i in 0..depth {
        json = format!(r#"{{"type": "http", "identifier": "d{i}", "children": [{json}]}}"#);
    }
    serde_json::from_str(&format!(r#"{{"dependencies": [{json}]}}"#)).expect("valid deeply nested match config")
}

#[test]
fn build_dependencies_depth_within_limit_builds_successfully() {
    let config = deeply_nested_dependency_config(MAX_CHILDREN_DEPTH - 1);
    assert!(build_dependencies(&config, None).is_ok());
}

#[test]
fn build_dependencies_depth_exceeds_limit_returns_err() {
    let config = deeply_nested_dependency_config(MAX_CHILDREN_DEPTH + 1);
    match build_dependencies(&config, None) {
        Err(e) => assert!(e.contains("max depth"), "unexpected error: {e}"),
        Ok(_) => panic!("expected depth limit to be enforced"),
    }
}

#[tokio::test]
async fn build_components_async_nested_children_returns_only_root_components() {
    let config = nested_component_config();
    let components = build_components_async(&config).await.expect("nested component config builds");
    assert_eq!(components.len(), 1);
}

#[tokio::test]
async fn build_components_async_exec_variant_dispatches_to_builder() {
    let config = exec_component_config();
    let components = build_components_async(&config).await.expect("exec variant builds");
    assert_eq!(components.len(), 1);
}

#[tokio::test]
async fn build_match_async_full_config_registers_playbook_with_overrides() {
    let config = full_match_config_with_playbook();
    let result = build_match_async(&config).await;
    if let Err(e) = result {
        panic!("expected match to build: {e}");
    }
}
