use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena_oracledb::{OracleDependency, OracleImpl};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

struct AlwaysReadyCheck;

#[async_trait]
impl ReadinessCheck for AlwaysReadyCheck {
    async fn is_ready(&self, _identifier: &str, _target: &str, _timeout_ms: u64) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Default)]
struct RecorderInner {
    start_calls: Mutex<u32>,
    stop_calls: Mutex<u32>,
    sqlplus_calls: Mutex<Vec<(String, String, String)>>,
}

#[derive(Clone, Default)]
struct RecordingOracleImpl {
    inner: Arc<RecorderInner>,
}

impl RecordingOracleImpl {
    fn start_call_count(&self) -> u32 {
        *self.inner.start_calls.lock().expect("start_calls lock")
    }

    fn stop_call_count(&self) -> u32 {
        *self.inner.stop_calls.lock().expect("stop_calls lock")
    }

    fn sqlplus_calls_as_user(&self, username: &str) -> usize {
        self.inner
            .sqlplus_calls
            .lock()
            .expect("sqlplus_calls lock")
            .iter()
            .filter(|(u, _, _)| u == username)
            .count()
    }

    fn any_sqlplus_call_contains(&self, needle: &str) -> bool {
        self.inner
            .sqlplus_calls
            .lock()
            .expect("sqlplus_calls lock")
            .iter()
            .any(|(_, _, script)| script.contains(needle))
    }
}

#[async_trait]
impl OracleImpl for RecordingOracleImpl {
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
        *self.inner.start_calls.lock().expect("start_calls lock") += 1;
    }

    async fn stop(&self) {
        *self.inner.stop_calls.lock().expect("stop_calls lock") += 1;
    }

    fn connection_string(&self) -> Option<String> {
        Some("//localhost:1521/FREEPDB1".to_string())
    }

    fn host_address(&self) -> Option<String> {
        Some("127.0.0.1:1521".to_string())
    }

    async fn run_sqlplus(&self, username: &str, password: &str, script: &str) -> Result<String, String> {
        self.inner
            .sqlplus_calls
            .lock()
            .expect("sqlplus_calls lock")
            .push((username.to_string(), password.to_string(), script.to_string()));

        if script.contains("SELECT 1 FROM DUAL") {
            return Ok("1\n".to_string());
        }

        Ok(String::new())
    }
}

struct FailingSqlReadinessOracleImpl;

#[async_trait]
impl OracleImpl for FailingSqlReadinessOracleImpl {
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
        Err("simulated sql readiness failure".to_string())
    }
}

#[tokio::test]
async fn start_called_once_starts_the_container() {
    let recorder = RecordingOracleImpl::default();
    let mut dep = OracleDependency::builder("start-once")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .build();

    dep.start().await;

    assert_eq!(recorder.start_call_count(), 1);
}

#[tokio::test]
#[should_panic(expected = "sql-level readiness check failed")]
async fn start_sql_readiness_check_failure_panics() {
    let mut dep = OracleDependency::builder("start-sql-readiness-fails")
        .with_impl(FailingSqlReadinessOracleImpl)
        .with_readiness_check(AlwaysReadyCheck)
        .build();

    dep.start().await;
}

#[tokio::test]
async fn start_called_twice_only_starts_container_once() {
    let recorder = RecordingOracleImpl::default();
    let mut dep = OracleDependency::builder("start-twice")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .build();

    dep.start().await;
    dep.start().await;

    assert_eq!(recorder.start_call_count(), 1);
}

#[tokio::test]
async fn start_with_startup_scripts_runs_them_as_app_user() {
    let recorder = RecordingOracleImpl::default();
    let mut dep = OracleDependency::builder("start-with-scripts")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .with_database_username("app_owner")
        .with_startup_sql_scripts(vec!["CREATE TABLE widgets (id NUMBER);".to_string()])
        .build();

    dep.start().await;

    assert!(recorder.any_sqlplus_call_contains("CREATE TABLE widgets"));
    assert!(recorder.sqlplus_calls_as_user("app_owner") >= 1);
}

#[tokio::test]
async fn start_snapshots_managed_tables_via_user_tables_query() {
    let recorder = RecordingOracleImpl::default();
    let mut dep = OracleDependency::builder("start-snapshot")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .build();

    dep.start().await;

    assert!(recorder.any_sqlplus_call_contains("USER_TABLES"));
    assert!(dep.managed_tables().is_empty());
}

#[tokio::test]
async fn soft_reset_without_startup_scripts_does_not_touch_admin_user() {
    let recorder = RecordingOracleImpl::default();
    let mut dep = OracleDependency::builder("soft-reset-noop")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .build();

    dep.start().await;
    dep.soft_reset().await;

    assert_eq!(recorder.sqlplus_calls_as_user("system"), 0);
}

#[tokio::test]
async fn soft_reset_with_startup_scripts_recreates_app_user_as_admin() {
    let recorder = RecordingOracleImpl::default();
    let mut dep = OracleDependency::builder("soft-reset-recreates")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .with_startup_sql_scripts(vec!["CREATE TABLE widgets (id NUMBER);".to_string()])
        .build();

    dep.start().await;
    dep.soft_reset().await;

    assert!(recorder.sqlplus_calls_as_user("system") >= 2);
    assert!(recorder.any_sqlplus_call_contains("DROP USER"));
    assert!(recorder.any_sqlplus_call_contains("CREATE USER"));
}

#[tokio::test]
async fn soft_reset_before_start_is_noop() {
    let recorder = RecordingOracleImpl::default();
    let dep = OracleDependency::builder("soft-reset-before-start")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .with_startup_sql_scripts(vec!["CREATE TABLE widgets (id NUMBER);".to_string()])
        .build();

    dep.soft_reset().await;

    assert_eq!(recorder.sqlplus_calls_as_user("system"), 0);
}

#[tokio::test]
async fn hard_reset_restarts_container_and_reruns_startup_scripts() {
    let recorder = RecordingOracleImpl::default();
    let mut dep = OracleDependency::builder("hard-reset")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .with_startup_sql_scripts(vec!["CREATE TABLE widgets (id NUMBER);".to_string()])
        .build();

    dep.start().await;
    dep.hard_reset().await;

    assert_eq!(recorder.start_call_count(), 2);
    assert_eq!(recorder.stop_call_count(), 1);
}

#[tokio::test]
async fn hard_reset_before_start_is_noop() {
    let recorder = RecordingOracleImpl::default();
    let mut dep = OracleDependency::builder("hard-reset-before-start")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .build();

    dep.hard_reset().await;

    assert_eq!(recorder.start_call_count(), 0);
    assert_eq!(recorder.stop_call_count(), 0);
}

#[tokio::test]
async fn stop_before_start_does_not_panic() {
    let recorder = RecordingOracleImpl::default();
    let mut dep = OracleDependency::builder("stop-before-start")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .build();

    dep.stop().await;

    assert_eq!(recorder.stop_call_count(), 1);
}

#[test]
fn identifier_includes_builder_prefix_and_given_name() {
    let dep = OracleDependency::builder("my-oracle-dep").build();

    assert!(dep.identifier().starts_with("arena-oracledb-my-oracle-dep-"));
}

#[test]
fn as_any_downcasts_to_oracle_dependency() {
    let dep = OracleDependency::builder("downcast-test").build();

    let any_ref = dep.as_any();
    assert!(any_ref.downcast_ref::<OracleDependency>().is_some());
}

#[test]
fn children_empty_by_default() {
    let dep = OracleDependency::builder("no-children").build();

    assert!(dep.children().is_empty());
}

#[derive(Clone, Default)]
struct RecordingChildDependency {
    log: Arc<Mutex<Vec<&'static str>>>,
}

#[async_trait]
impl RunnableDependency for RecordingChildDependency {
    fn identifier(&self) -> &str {
        "oracle-child"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn start(&mut self) {
        self.log.lock().expect("log lock").push("child-start");
    }

    async fn stop(&mut self) {
        self.log.lock().expect("log lock").push("child-stop");
    }

    fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}
    fn children(&self) -> &[Dependency] {
        &[]
    }
    fn children_mut(&mut self) -> &mut [Dependency] {
        &mut []
    }

    async fn soft_reset(&self) {}
    async fn hard_reset(&mut self) {}
}

#[test]
fn add_child_reflects_in_children_and_children_mut() {
    let mut dep = OracleDependency::builder("add-child").build();

    dep.add_child(Box::new(RecordingChildDependency::default()));

    assert_eq!(dep.children().len(), 1);
    assert_eq!(dep.children_mut().len(), 1);
}

#[tokio::test]
async fn start_with_children_starts_children_before_container() {
    let recorder = RecordingOracleImpl::default();
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut dep = OracleDependency::builder("start-with-children")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .build();
    dep.add_child(Box::new(RecordingChildDependency { log: log.clone() }));

    dep.start().await;

    assert_eq!(log.lock().expect("log lock").as_slice(), &["child-start"]);
    assert_eq!(recorder.start_call_count(), 1);
}

#[tokio::test]
async fn stop_running_with_children_stops_children_in_reverse_order() {
    let recorder = RecordingOracleImpl::default();
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut dep = OracleDependency::builder("stop-with-children")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .build();
    dep.add_child(Box::new(RecordingChildDependency { log: log.clone() }));

    dep.start().await;
    log.lock().expect("log lock").clear();
    dep.stop().await;

    assert_eq!(log.lock().expect("log lock").as_slice(), &["child-stop"]);
    assert_eq!(recorder.stop_call_count(), 1);
}

#[tokio::test]
async fn execute_runs_sql_as_database_user() {
    let recorder = RecordingOracleImpl::default();
    let mut dep = OracleDependency::builder("execute-as-user")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .with_database_username("app_owner")
        .build();
    dep.start().await;

    dep.execute("CREATE TABLE widgets (id NUMBER);").await;

    assert!(recorder.any_sqlplus_call_contains("CREATE TABLE widgets"));
    assert!(recorder.sqlplus_calls_as_user("app_owner") >= 1);
}

#[tokio::test]
async fn query_scalar_returns_parsed_value() {
    let recorder = RecordingOracleImpl::default();
    let mut dep = OracleDependency::builder("query-scalar")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .build();
    dep.start().await;

    let value = dep.query_scalar("SELECT 1 FROM DUAL").await;

    assert_eq!(value, 1);
}
