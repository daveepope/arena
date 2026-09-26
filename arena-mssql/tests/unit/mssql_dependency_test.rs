use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena::lifecycle::{Fault, RunnableState};
use arena_mssql::{MssqlDependency, MssqlImpl};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const CLOSED_PORT_ADMIN: &str =
    "Server=tcp:127.0.0.1,1;Database=master;User Id=sa;Password=pw;TrustServerCertificate=True;";
const CLOSED_PORT_DATABASE: &str =
    "Server=tcp:127.0.0.1,1;Database=readings;User Id=sa;Password=pw;TrustServerCertificate=True;";

struct FakeMssqlImpl {
    connection_string: Option<String>,
    admin_connection_string: Option<String>,
    force_stop_confirms_removal: bool,
    starts: Arc<AtomicUsize>,
    stops: Arc<AtomicUsize>,
    releases: Arc<AtomicUsize>,
}

impl FakeMssqlImpl {
    fn new() -> Self {
        Self {
            connection_string: None,
            admin_connection_string: None,
            force_stop_confirms_removal: true,
            starts: Arc::new(AtomicUsize::new(0)),
            stops: Arc::new(AtomicUsize::new(0)),
            releases: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn without_confirmed_removal(mut self) -> Self {
        self.force_stop_confirms_removal = false;
        self
    }
}

#[async_trait]
impl MssqlImpl for FakeMssqlImpl {
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
        self.connection_string = Some(CLOSED_PORT_DATABASE.to_string());
        self.admin_connection_string = Some(CLOSED_PORT_ADMIN.to_string());
        self.starts.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.connection_string = None;
        self.admin_connection_string = None;
        self.stops.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn force_stop(&mut self) -> bool {
        self.release();
        self.force_stop_confirms_removal
    }

    fn release(&mut self) {
        self.connection_string = None;
        self.admin_connection_string = None;
        self.releases.fetch_add(1, Ordering::SeqCst);
    }

    fn connection_string(&self) -> Option<&str> {
        self.connection_string.as_deref()
    }

    fn admin_connection_string(&self) -> Option<&str> {
        self.admin_connection_string.as_deref()
    }
}

struct PassingReadinessCheck;

#[async_trait]
impl ReadinessCheck for PassingReadinessCheck {
    async fn is_ready(&self, _: &str, _: &str, _: u64) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Default)]
struct ChildCalls {
    released: usize,
    force_stopped: usize,
}

struct FakeChildDependency {
    calls: Arc<Mutex<ChildCalls>>,
}

#[async_trait]
impl RunnableDependency for FakeChildDependency {
    fn identifier(&self) -> &str {
        "mssql-child"
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

    async fn start(&mut self) -> Result<(), Fault> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        Ok(())
    }

    async fn force_stop(&mut self) {
        self.calls.lock().unwrap().force_stopped += 1;
    }

    fn release(&mut self) {
        self.calls.lock().unwrap().released += 1;
    }

    fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}

    fn children(&self) -> &[Dependency] {
        &[]
    }

    fn children_mut(&mut self) -> &mut [Dependency] {
        &mut []
    }

    async fn soft_reset(&self) -> Result<(), Fault> {
        Ok(())
    }

    async fn hard_reset(&mut self) -> Result<(), Fault> {
        Ok(())
    }
}

fn setup_dependency(identifier: &str, mssql_impl: FakeMssqlImpl) -> MssqlDependency {
    MssqlDependency::builder(identifier)
        .with_database_name("master")
        .with_connect_timeout(Duration::from_millis(200))
        .with_impl(mssql_impl)
        .with_readiness_check(PassingReadinessCheck)
        .build()
}

#[tokio::test]
async fn release_started_dependency_releases_container_and_children() {
    let mssql_impl = FakeMssqlImpl::new();
    let releases = mssql_impl.releases.clone();
    let calls = Arc::new(Mutex::new(ChildCalls::default()));
    let mut dep = setup_dependency("mssql-release", mssql_impl);
    dep.add_child(Box::new(FakeChildDependency {
        calls: calls.clone(),
    }));

    dep.start().await.expect("start should succeed");
    dep.release();

    assert_eq!(dep.state(), RunnableState::Stopped);
    assert_eq!(releases.load(Ordering::SeqCst), 1);
    assert_eq!(calls.lock().unwrap().released, 1);
}

#[tokio::test]
async fn force_stop_repeated_unconfirmed_removal_records_one_fault() {
    let calls = Arc::new(Mutex::new(ChildCalls::default()));
    let mut dep = setup_dependency(
        "mssql-force-stop",
        FakeMssqlImpl::new().without_confirmed_removal(),
    );
    dep.add_child(Box::new(FakeChildDependency {
        calls: calls.clone(),
    }));

    dep.force_stop().await;
    dep.force_stop().await;

    assert_eq!(dep.state(), RunnableState::Faulted);
    assert_eq!(dep.faults().len(), 1);
    assert_eq!(calls.lock().unwrap().force_stopped, 2);
}

#[tokio::test]
async fn soft_reset_not_started_dependency_returns_ok() {
    let dep = MssqlDependency::builder("mssql-soft-reset-idle")
        .with_database_name("master")
        .with_startup_sql_scripts(vec!["select 1;".to_string()])
        .with_connect_timeout(Duration::from_millis(200))
        .with_impl(FakeMssqlImpl::new())
        .with_readiness_check(PassingReadinessCheck)
        .build();

    assert!(dep.soft_reset().await.is_ok());
}

#[tokio::test]
async fn soft_reset_without_startup_scripts_returns_ok() {
    let mut dep = setup_dependency("mssql-soft-reset-no-scripts", FakeMssqlImpl::new());
    dep.start().await.expect("start should succeed");

    assert!(dep.soft_reset().await.is_ok());
}

#[tokio::test]
async fn hard_reset_started_dependency_restarts_container() {
    let mssql_impl = FakeMssqlImpl::new();
    let starts = mssql_impl.starts.clone();
    let stops = mssql_impl.stops.clone();
    let mut dep = setup_dependency("mssql-hard-reset", mssql_impl);
    dep.start().await.expect("start should succeed");

    dep.hard_reset().await.expect("hard reset should succeed");

    assert_eq!(starts.load(Ordering::SeqCst), 2);
    assert_eq!(stops.load(Ordering::SeqCst), 1);
    assert_eq!(dep.state(), RunnableState::Started);
}

#[tokio::test]
async fn start_unreachable_database_returns_fault() {
    let mut dep = MssqlDependency::builder("mssql-unreachable-db")
        .with_database_name("readings")
        .with_connect_timeout(Duration::from_millis(200))
        .with_impl(FakeMssqlImpl::new())
        .with_readiness_check(PassingReadinessCheck)
        .build();

    let fault = dep.start().await.expect_err("start should fault");

    assert!(
        fault.message.contains("ensure database exists"),
        "unexpected fault: {}",
        fault.message
    );
    assert_eq!(dep.faults().len(), 1);
    assert_eq!(dep.state(), RunnableState::Stopped);
}

#[tokio::test]
async fn start_unreachable_startup_scripts_returns_fault() {
    let mut dep = MssqlDependency::builder("mssql-unreachable-scripts")
        .with_database_name("master")
        .with_startup_sql_scripts(vec!["select 1;".to_string()])
        .with_connect_timeout(Duration::from_millis(200))
        .with_impl(FakeMssqlImpl::new())
        .with_readiness_check(PassingReadinessCheck)
        .build();

    let fault = dep.start().await.expect_err("start should fault");

    assert!(
        fault.message.contains("connect for startup scripts"),
        "unexpected fault: {}",
        fault.message
    );
    assert_eq!(dep.faults().len(), 1);
    assert_eq!(dep.state(), RunnableState::Stopped);
}
