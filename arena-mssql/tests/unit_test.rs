use arena::lifecycle::{Fault, RunnableState};
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena_mssql::{MssqlDependency, MssqlImpl, DEFAULT_CONNECT_TIMEOUT};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    MssqlStart,
    MssqlStop,
    ReadinessCheck,
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
        true
    }
    fn release(&mut self) {}


    fn connection_string(&self) -> Option<&str> {
        self.conn_str.as_deref()
    }

    fn admin_connection_string(&self) -> Option<&str> {
        self.admin_conn_str.as_deref()
    }
}

struct FakeReadinessCheck {
    events: Arc<Mutex<Vec<Event>>>,
    last_identifier: Arc<Mutex<Option<String>>>,
    last_connection_string: Arc<Mutex<Option<String>>>,
    last_timeout_ms: Arc<Mutex<Option<u64>>>,
}

#[async_trait]
impl ReadinessCheck for FakeReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        connection_string: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        self.events.lock().unwrap().push(Event::ReadinessCheck);
        *self.last_identifier.lock().unwrap() = Some(identifier.to_string());
        *self.last_connection_string.lock().unwrap() = Some(connection_string.to_string());
        *self.last_timeout_ms.lock().unwrap() = Some(timeout_ms);
        Ok(())
    }
}

struct FailingReadinessCheck;

#[async_trait]
impl ReadinessCheck for FailingReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _connection_string: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Err("readiness failed".to_string())
    }
}

#[tokio::test]
async fn start_stop_happy_path_records_events() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let last_identifier = Arc::new(Mutex::new(None::<String>));
    let last_connection_string = Arc::new(Mutex::new(None::<String>));
    let last_timeout_ms = Arc::new(Mutex::new(None::<u64>));

    let mut mssql = MssqlDependency::builder("mssql")
        .with_database_name("master")
        .with_impl(FakeMssqlImpl {
            conn_str: None,
            admin_conn_str: None,
            events: events.clone(),
        })
        .with_readiness_check(FakeReadinessCheck {
            events: events.clone(),
            last_identifier: last_identifier.clone(),
            last_connection_string: last_connection_string.clone(),
            last_timeout_ms: last_timeout_ms.clone(),
        })
        .build();

    let outcome = std::panic::AssertUnwindSafe(async {
        mssql.start().await.expect("start should succeed");
        mssql.stop().await.expect("stop should succeed");
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_ok(), "expected start/stop not to panic");

    let got = events.lock().unwrap().clone();
    assert_eq!(
        got,
        vec![Event::MssqlStart, Event::ReadinessCheck, Event::MssqlStop]
    );

    assert_eq!(
        last_identifier.lock().unwrap().as_deref(),
        Some(mssql.identifier.as_str())
    );
    assert_eq!(
        last_connection_string.lock().unwrap().as_deref(),
        Some("Server=tcp:127.0.0.1,1433;Database=master;User Id=sa;Password=pw;TrustServerCertificate=True;")
    );
    assert_eq!(*last_timeout_ms.lock().unwrap(), Some(30_000));
}

#[tokio::test]
async fn start_readiness_err_panics_after_impl_start() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = MssqlDependency::builder("mssql")
        .with_database_name("master")
        .with_impl(FakeMssqlImpl {
            conn_str: None,
            admin_conn_str: None,
            events: events.clone(),
        })
        .with_readiness_check(FailingReadinessCheck)
        .build();

    let outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await.expect("start should succeed");
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_err());
    assert_eq!(events.lock().unwrap().as_slice(), &[Event::MssqlStart]);
}

#[tokio::test]
async fn builder_default_connect_timeout_matches_constant() {
    let dep = MssqlDependency::builder("mssql_defaults")
        .with_impl(FakeMssqlImpl {
            conn_str: None,
            admin_conn_str: None,
            events: Arc::new(Mutex::new(Vec::new())),
        })
        .build();
    assert_eq!(dep.connect_timeout(), Some(DEFAULT_CONNECT_TIMEOUT));
}

#[tokio::test]
async fn builder_with_connect_timeout_overrides_default() {
    let custom = Duration::from_millis(25);
    let dep = MssqlDependency::builder("mssql_custom")
        .with_impl(FakeMssqlImpl {
            conn_str: None,
            admin_conn_str: None,
            events: Arc::new(Mutex::new(Vec::new())),
        })
        .with_connect_timeout(custom)
        .build();
    assert_eq!(dep.connect_timeout(), Some(custom));
}

#[tokio::test]
async fn builder_without_connect_timeout_disables_bounding() {
    let dep = MssqlDependency::builder("mssql_unbounded")
        .with_impl(FakeMssqlImpl {
            conn_str: None,
            admin_conn_str: None,
            events: Arc::new(Mutex::new(Vec::new())),
        })
        .without_connect_timeout()
        .build();
    assert_eq!(dep.connect_timeout(), None);
}

struct NoopChildDependency;

#[async_trait]
impl RunnableDependency for NoopChildDependency {
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

    async fn force_stop(&mut self) {}
    fn release(&mut self) {}


    async fn start(&mut self) -> Result<(), Fault> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        Ok(())
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

#[test]
fn identifier_as_any_and_children_reflect_dependency_state() {
    let mut dep = MssqlDependency::builder("mssql-accessors")
        .with_database_name("master")
        .with_impl(FakeMssqlImpl {
            conn_str: None,
            admin_conn_str: None,
            events: Arc::new(Mutex::new(Vec::new())),
        })
        .build();

    assert!(dep.identifier().contains("mssql-accessors"));
    assert!(dep.as_any().downcast_ref::<MssqlDependency>().is_some());
    assert!(dep.as_any_mut().downcast_mut::<MssqlDependency>().is_some());
    assert!(dep.children().is_empty());
    assert_eq!(dep.database_name(), "master");
    assert!(dep.managed_tables().is_empty());
    assert_eq!(dep.connection_string(), None);

    dep.add_child(Box::new(NoopChildDependency));

    assert_eq!(dep.children().len(), 1);
    assert_eq!(dep.children_mut().len(), 1);
}

struct AlwaysOkReadinessCheck;

#[async_trait]
impl ReadinessCheck for AlwaysOkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _connection_string: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn playbook_after_start_uses_connection_string() {
    let mut dep = MssqlDependency::builder("mssql-playbook")
        .with_database_name("master")
        .with_impl(FakeMssqlImpl {
            conn_str: None,
            admin_conn_str: None,
            events: Arc::new(Mutex::new(Vec::new())),
        })
        .with_readiness_check(AlwaysOkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    assert_eq!(
        dep.connection_string(),
        Some("Server=tcp:127.0.0.1,1433;Database=fake;User Id=sa;Password=pw;TrustServerCertificate=True;")
    );
    let _playbook = dep.playbook();
    dep.stop().await.expect("stop should succeed");
}
