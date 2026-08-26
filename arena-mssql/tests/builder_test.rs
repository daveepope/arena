use arena::dependency::RunnableDependency;
use arena_mssql::{MssqlDependency, MssqlEncryption};
use std::time::Duration;

struct NoopChildDependency;

#[async_trait::async_trait]
impl RunnableDependency for NoopChildDependency {
    fn identifier(&self) -> &str {
        "builder-child"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    async fn start(&mut self) {}
    async fn stop(&mut self) {}
    fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}
    fn children(&self) -> &[arena::dependency::Dependency] {
        &[]
    }
    fn children_mut(&mut self) -> &mut [arena::dependency::Dependency] {
        &mut []
    }
    async fn soft_reset(&self) {}
    async fn hard_reset(&mut self) {}
}

#[test]
fn build_no_overrides_uses_documented_defaults() {
    let dep = MssqlDependency::builder("builder-defaults").build();

    assert_eq!(dep.database_name(), "arena_db");
    assert_eq!(dep.connect_timeout(), Some(Duration::from_secs(3)));
    assert!(dep.connection_string().is_none());
    assert!(dep.admin_connection_string().is_none());
    assert!(dep.managed_tables().is_empty());
}

#[test]
fn build_with_database_name_overrides_default() {
    let dep = MssqlDependency::builder("builder-db-name")
        .with_database_name("custom_db")
        .build();

    assert_eq!(dep.database_name(), "custom_db");
}

#[test]
fn build_with_image_sets_image_tag() {
    let dep = MssqlDependency::builder("builder-with-image")
        .with_image("tag-via-with-image")
        .build();

    assert!(dep.identifier().starts_with("arena-mssql-builder-with-image"));
}

#[test]
fn build_with_container_tag_sets_image_tag() {
    let dep = MssqlDependency::builder("builder-with-container-tag")
        .with_container_tag("tag-via-container-tag")
        .build();

    assert!(dep.identifier().starts_with("arena-mssql-builder-with-container-tag"));
}

#[test]
fn build_with_child_dependencies_adds_children() {
    let dep = MssqlDependency::builder("builder-children")
        .with_child_dependencies(vec![Box::new(NoopChildDependency)])
        .build();

    assert_eq!(dep.children().len(), 1);
}

#[test]
fn build_without_connect_timeout_disables_timeout() {
    let dep = MssqlDependency::builder("builder-no-timeout")
        .without_connect_timeout()
        .build();

    assert_eq!(dep.connect_timeout(), None);
}

#[test]
fn build_with_connect_timeout_sets_custom_value() {
    let custom = Duration::from_secs(9);
    let dep = MssqlDependency::builder("builder-custom-timeout")
        .with_connect_timeout(custom)
        .build();

    assert_eq!(dep.connect_timeout(), Some(custom));
}

#[test]
fn build_with_encryption_does_not_panic() {
    let dep = MssqlDependency::builder("builder-encryption")
        .with_encryption(MssqlEncryption::On)
        .build();

    assert!(dep.identifier().starts_with("arena-mssql-builder-encryption"));
}

#[test]
fn build_with_all_metadata_options_does_not_panic() {
    let dep = MssqlDependency::builder("builder-metadata")
        .with_port(5555)
        .with_database_username("custom_user")
        .with_database_password("custom_pw")
        .with_image_name("custom/image")
        .with_image_tag("custom-tag")
        .with_container_name("custom-container")
        .with_network("custom-network")
        .with_startup_sql_scripts(vec!["SELECT 1;".to_string()])
        .build();

    assert!(dep.identifier().starts_with("arena-mssql-builder-metadata"));
    assert!(dep.managed_tables().is_empty());
}
