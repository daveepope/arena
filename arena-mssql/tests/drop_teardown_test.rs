use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_mssql::{MssqlDependency, MssqlImpl};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    MssqlStart,
    MssqlStop,
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
    ) {
        self.conn_str = Some(
            "Server=tcp:127.0.0.1,1433;Database=fake;User Id=sa;Password=pw;TrustServerCertificate=True;"
                .to_string(),
        );
        self.admin_conn_str = Some(
            "Server=tcp:127.0.0.1,1433;Database=master;User Id=sa;Password=pw;TrustServerCertificate=True;"
                .to_string(),
        );
        self.events.lock().unwrap().push(Event::MssqlStart);
    }

    async fn stop(&mut self) {
        self.conn_str = None;
        self.admin_conn_str = None;
        self.events.lock().unwrap().push(Event::MssqlStop);
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

struct PanickingMssqlReadinessCheck;

#[async_trait]
impl ReadinessCheck for PanickingMssqlReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _connection_string: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        panic!("readiness probe failed");
    }
}

fn mssql_stop_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::MssqlStop))
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
    dep.start().await;
    dep.stop().await;
    drop(dep);
    assert_eq!(mssql_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn drop_running_dep_invokes_full_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_mssql(events.clone());
    dep.start().await;
    drop(dep);
    assert_eq!(mssql_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn start_panic_then_drop_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = MssqlDependency::builder("mssql-drop")
        .with_database_name("master")
        .with_impl(FakeMssqlImpl {
            conn_str: None,
            admin_conn_str: None,
            events: events.clone(),
        })
        .with_readiness_check(PanickingMssqlReadinessCheck)
        .build();

    let start_outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await;
    })
    .catch_unwind()
    .await;
    assert!(start_outcome.is_err());
    assert_eq!(events.lock().unwrap().as_slice(), &[Event::MssqlStart]);

    drop(dep);
    assert_eq!(mssql_stop_count(&events.lock().unwrap()), 1);
}
