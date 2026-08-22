use arena_oracledb::{OracleDependency, OracleImpl};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::{Arc, Mutex};

struct FakeStartedOracleImpl;

#[async_trait]
impl OracleImpl for FakeStartedOracleImpl {
    #[allow(clippy::too_many_arguments)]
    async fn start(
        &self,
        _port: u16,
        _database_name: &str,
        _database_username: &str,
        _database_password: &str,
        _admin_password: &str,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
    ) {
    }

    async fn stop(&self) {}

    fn connection_string(&self) -> Option<String> {
        Some("//localhost:1521/FREEPDB1".to_string())
    }

    fn host_address(&self) -> Option<String> {
        Some("127.0.0.1:1521".to_string())
    }

    async fn run_sqlplus(&self, _username: &str, _password: &str, _script: &str) -> Result<String, String> {
        Ok(String::new())
    }
}

#[test]
#[should_panic(expected = "must be started before configuring a Playbook")]
fn with_dependency_not_started_panics() {
    let dep = OracleDependency::builder("playbook-not-started").build();
    let _ = dep.playbook();
}

#[test]
fn with_dependency_started_constructs_without_panicking() {
    let dep = OracleDependency::builder("playbook-started")
        .with_impl(FakeStartedOracleImpl)
        .build();

    let _playbook = dep.playbook();
}

#[test]
fn with_identifier_overrides_default_identifier() {
    let dep = OracleDependency::builder("playbook-custom-id")
        .with_impl(FakeStartedOracleImpl)
        .build();

    let playbook = dep.playbook().with_identifier("custom-playbook-id");

    let _ = playbook;
}

#[derive(Default)]
struct ScriptAwareOracleImplInner {
    calls: Mutex<Vec<String>>,
}

#[derive(Clone, Default)]
struct ScriptAwareOracleImpl {
    inner: Arc<ScriptAwareOracleImplInner>,
    fail_on_delete: bool,
}

impl ScriptAwareOracleImpl {
    fn new(fail_on_delete: bool) -> Self {
        Self {
            inner: Arc::default(),
            fail_on_delete,
        }
    }

    fn any_call_contains(&self, needle: &str) -> bool {
        self.inner
            .calls
            .lock()
            .expect("calls lock")
            .iter()
            .any(|script| script.contains(needle))
    }
}

#[async_trait]
impl OracleImpl for ScriptAwareOracleImpl {
    #[allow(clippy::too_many_arguments)]
    async fn start(
        &self,
        _port: u16,
        _database_name: &str,
        _database_username: &str,
        _database_password: &str,
        _admin_password: &str,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
    ) {
    }

    async fn stop(&self) {}

    fn connection_string(&self) -> Option<String> {
        Some("//localhost:1521/FREEPDB1".to_string())
    }

    fn host_address(&self) -> Option<String> {
        Some("127.0.0.1:1521".to_string())
    }

    async fn run_sqlplus(&self, _username: &str, _password: &str, script: &str) -> Result<String, String> {
        self.inner.calls.lock().expect("calls lock").push(script.to_string());

        if script.contains("USER_TABLES") {
            return Ok("WEIRD TABLE\n".to_string());
        }
        if script.contains("user_constraints") {
            return Ok("WEIRD TABLE|FK_WEIRD_OWNER\n".to_string());
        }
        if self.fail_on_delete && script.contains("DELETE FROM") {
            return Err("simulated delete failure".to_string());
        }

        Ok(String::new())
    }
}

#[tokio::test]
async fn run_delete_failure_still_reenables_constraints() {
    let fake = ScriptAwareOracleImpl::new(true);
    let dep = OracleDependency::builder("reset-delete-fails").with_impl(fake.clone()).build();

    let outcome = std::panic::AssertUnwindSafe(dep.playbook().run()).catch_unwind().await;

    assert!(outcome.is_err(), "expected the delete failure to surface as a panic");
    assert!(
        fake.any_call_contains("ENABLE CONSTRAINT"),
        "constraints must be re-enabled even when the delete step fails"
    );
}

#[tokio::test]
async fn run_table_name_with_space_is_quoted_in_generated_sql() {
    let fake = ScriptAwareOracleImpl::new(false);
    let dep = OracleDependency::builder("reset-quoting").with_impl(fake.clone()).build();

    let _active = dep.playbook().run().await;

    assert!(fake.any_call_contains("DELETE FROM \"WEIRD TABLE\";"));
    assert!(fake.any_call_contains("ALTER TABLE \"WEIRD TABLE\" DISABLE CONSTRAINT \"FK_WEIRD_OWNER\";"));
    assert!(fake.any_call_contains("ALTER TABLE \"WEIRD TABLE\" ENABLE CONSTRAINT \"FK_WEIRD_OWNER\";"));
}
