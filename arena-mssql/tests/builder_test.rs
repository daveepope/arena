use arena::lifecycle::{Fault, RunnableState};
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
    fn state(&self) -> RunnableState {
        RunnableState::NotStarted
    }

    fn faults(&self) -> &[Fault] {
        &[]
    }

    async fn force_stop(&mut self) {}
    fn release(&mut self) {}


    async fn start(&mut self) -> Result<(), Fault> {
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), Fault> {
        Ok(())
    }
    fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}
    fn children(&self) -> &[arena::dependency::Dependency] {
        &[]
    }
    fn children_mut(&mut self) -> &mut [arena::dependency::Dependency] {
        &mut []
    }
    async fn soft_reset(&self) -> Result<(), Fault> {
        Ok(())
    }
    async fn hard_reset(&mut self) -> Result<(), Fault> {
        Ok(())
    }
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


#[derive(Clone, Default)]
struct ExpiryRecordingImpl {
    expiry: std::sync::Arc<std::sync::Mutex<Option<Option<std::time::Duration>>>>,
}

#[async_trait::async_trait]
impl arena_mssql::MssqlImpl for ExpiryRecordingImpl {
    fn set_expiry(&mut self, expiry: Option<std::time::Duration>) {
        *self.expiry.lock().unwrap() = Some(expiry);
    }
    #[allow(clippy::too_many_arguments)]
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
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        true
    }
    fn release(&mut self) {}
    fn connection_string(&self) -> Option<&str> {
        None
    }
    fn admin_connection_string(&self) -> Option<&str> {
        None
    }
}

#[test]
fn build_no_expiry_override_uses_default_expiry() {
    let recorder = ExpiryRecordingImpl::default();
    let _dep = MssqlDependency::builder("orders").with_impl(recorder.clone()).build();

    assert_eq!(
        *recorder.expiry.lock().unwrap(),
        Some(Some(arena_container::expiry::DEFAULT_EXPIRY))
    );
}

#[test]
fn build_with_expiry_uses_given_expiry() {
    let recorder = ExpiryRecordingImpl::default();
    let _dep = MssqlDependency::builder("orders")
        .with_impl(recorder.clone())
        .with_expiry(std::time::Duration::from_secs(30))
        .build();

    assert_eq!(
        *recorder.expiry.lock().unwrap(),
        Some(Some(std::time::Duration::from_secs(30)))
    );
}

#[test]
fn build_without_expiry_disables_expiry() {
    let recorder = ExpiryRecordingImpl::default();
    let _dep = MssqlDependency::builder("orders").with_impl(recorder.clone()).without_expiry().build();

    assert_eq!(*recorder.expiry.lock().unwrap(), Some(None));
}
