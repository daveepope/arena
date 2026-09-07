use arena::dependency::RunnableDependency;
use arena::lifecycle::RunnableState;
use arena::healthcheck::ReadinessCheck;
use arena_mssql::{MssqlDependency, MssqlImpl};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    MssqlStart,
    MssqlStop,
    MssqlForceStop,
    MssqlRelease,
}

struct FakeMssqlImpl {
    conn_str: Option<String>,
    admin_conn_str: Option<String>,
    events: Arc<Mutex<Vec<Event>>>,
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
        self.conn_str = Some(
            "Server=tcp:127.0.0.1,1433;Database=fake;User Id=sa;Password=pw;TrustServerCertificate=True;"
                .to_string(),
        );
        self.admin_conn_str = Some(
            "Server=tcp:127.0.0.1,1433;Database=master;User Id=sa;Password=pw;TrustServerCertificate=True;"
                .to_string(),
        );
        self.events.lock().unwrap().push(Event::MssqlStart);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.conn_str = None;
        self.admin_conn_str = None;
        self.events.lock().unwrap().push(Event::MssqlStop);
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        self.events.lock().unwrap().push(Event::MssqlForceStop);
        true
    }
    fn release(&mut self) {
        self.events.lock().unwrap().push(Event::MssqlRelease);
    }


    fn connection_string(&self) -> Option<&str> {
        self.conn_str.as_deref()
    }

    fn admin_connection_string(&self) -> Option<&str> {
        self.admin_conn_str.as_deref()
    }
}

struct OkReadinessCheck;

#[async_trait]
impl ReadinessCheck for OkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _connection_string: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

struct FailingMssqlReadinessCheck;

#[async_trait]
impl ReadinessCheck for FailingMssqlReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _connection_string: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Err("readiness probe failed".to_string())
    }
}

fn mssql_stop_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::MssqlStop))
        .count()
}

fn force_stop_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::MssqlForceStop))
        .count()
}

fn release_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::MssqlRelease))
        .count()
}

fn build_mssql(events: Arc<Mutex<Vec<Event>>>) -> MssqlDependency {
    MssqlDependency::builder("mssql-drop")
        .with_database_name("master")
        .with_impl(FakeMssqlImpl {
            conn_str: None,
            admin_conn_str: None,
            events,
        })
        .with_readiness_check(OkReadinessCheck)
        .build()
}

fn build_mssql_with_failing_readiness(events: Arc<Mutex<Vec<Event>>>) -> MssqlDependency {
    MssqlDependency::builder("mssql-drop")
        .with_database_name("master")
        .with_impl(FakeMssqlImpl {
            conn_str: None,
            admin_conn_str: None,
            events,
        })
        .with_readiness_check(FailingMssqlReadinessCheck)
        .build()
}

#[test]
fn drop_unstarted_dep_skips_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let dep = build_mssql(events.clone());
    drop(dep);
    assert_eq!(mssql_stop_count(&events.lock().unwrap()), 0);
}

#[tokio::test]
async fn stop_then_drop_single_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_mssql(events.clone());
    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");
    drop(dep);
    assert_eq!(mssql_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn drop_running_dependency_releases_container() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_mssql(events.clone());
    dep.start().await.expect("start should succeed");
    drop(dep);
    assert_eq!(release_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn start_readiness_failure_returns_fault_and_forces_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_mssql_with_failing_readiness(events.clone());

    let fault = dep.start().await.expect_err("dependency should fault");

    assert_eq!(fault.id, dep.identifier());
    assert_eq!(dep.state(), RunnableState::Stopped);
    assert_eq!(dep.faults().len(), 1);
    assert_eq!(force_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn start_readiness_failure_then_drop_does_not_force_stop_twice() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_mssql_with_failing_readiness(events.clone());

    let _fault = dep.start().await.expect_err("dependency should fault");
    drop(dep);

    assert_eq!(force_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn force_stop_called_twice_is_indistinguishable_from_once() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_mssql(events.clone());

    dep.start().await.expect("dependency should start");
    dep.force_stop().await;
    let after_first = dep.state();
    dep.force_stop().await;

    assert_eq!(after_first, RunnableState::Stopped);
    assert_eq!(dep.state(), RunnableState::Stopped);
    assert!(dep.faults().is_empty());
}
