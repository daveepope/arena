use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena::lifecycle::{Fault, RunnableState};
use arena_postgres::{PostgresDependency, PostgresImpl};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    PostgresStart,
    PostgresStop,
    PostgresForceStop,
    PostgresRelease,
}

struct FakePostgresImpl {
    conn_str: Option<String>,
    force_stop_confirms_removal: bool,
    events: Arc<Mutex<Vec<Event>>>,
}

#[async_trait]
impl PostgresImpl for FakePostgresImpl {
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
        self.conn_str = Some("postgres://127.0.0.1:5432/fake".to_string());
        self.events.lock().unwrap().push(Event::PostgresStart);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.conn_str = None;
        self.events.lock().unwrap().push(Event::PostgresStop);
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        self.events.lock().unwrap().push(Event::PostgresForceStop);
        self.release();
        self.force_stop_confirms_removal
    }

    fn release(&mut self) {
        self.conn_str = None;
        self.events.lock().unwrap().push(Event::PostgresRelease);
    }


    fn connection_string(&self) -> Option<&str> {
        self.conn_str.as_deref()
    }
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

fn build_dep(events: Arc<Mutex<Vec<Event>>>) -> PostgresDependency {
    build_dep_with_removal(events, true)
}

fn build_dep_with_removal(
    events: Arc<Mutex<Vec<Event>>>,
    force_stop_confirms_removal: bool,
) -> PostgresDependency {
    PostgresDependency::builder("postgres-reset")
        .with_impl(FakePostgresImpl {
            conn_str: None,
            force_stop_confirms_removal,
            events,
        })
        .with_readiness_check(AlwaysOkReadinessCheck)
        .build()
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
        "postgres-child"
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

#[tokio::test]
async fn soft_reset_not_running_returns_early() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let dep = build_dep(events.clone());

    dep.soft_reset().await.expect("soft reset should succeed");

    assert!(events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn soft_reset_running_no_scripts_returns_early() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_dep(events.clone());
    dep.start().await.expect("start should succeed");

    dep.soft_reset().await.expect("soft reset should succeed");

    dep.stop().await.expect("stop should succeed");
}

#[tokio::test]
async fn hard_reset_not_running_returns_early() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_dep(events.clone());

    dep.hard_reset().await.expect("hard reset should succeed");

    assert!(events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn hard_reset_running_restarts_container() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_dep(events.clone());
    dep.start().await.expect("start should succeed");

    dep.hard_reset().await.expect("hard reset should succeed");

    let got = events.lock().unwrap().clone();
    assert_eq!(
        got,
        vec![
            Event::PostgresStart,
            Event::PostgresStop,
            Event::PostgresStart,
        ]
    );

    dep.stop().await.expect("stop should succeed");
}

#[tokio::test]
async fn drop_needs_teardown_without_running_invokes_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_dep(events.clone());
    dep.start().await.expect("start should succeed");
    dep.hard_reset().await.expect("hard reset should succeed");
    dep.stop().await.expect("stop should succeed");

    drop(dep);

    let got = events.lock().unwrap().clone();
    assert_eq!(
        got,
        vec![
            Event::PostgresStart,
            Event::PostgresStop,
            Event::PostgresStart,
            Event::PostgresStop,
        ]
    );
}

#[tokio::test]
async fn release_started_dependency_releases_container_and_children() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let calls = Arc::new(Mutex::new(ChildCalls::default()));
    let mut dep = build_dep(events.clone());
    dep.add_child(Box::new(FakeChildDependency {
        calls: calls.clone(),
    }));
    dep.start().await.expect("start should succeed");

    dep.release();

    assert_eq!(dep.state(), RunnableState::Stopped);
    assert_eq!(calls.lock().unwrap().released, 1);
    assert!(events.lock().unwrap().contains(&Event::PostgresRelease));
}

#[tokio::test]
async fn force_stop_repeated_unconfirmed_removal_records_one_fault() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let calls = Arc::new(Mutex::new(ChildCalls::default()));
    let mut dep = build_dep_with_removal(events, false);
    dep.add_child(Box::new(FakeChildDependency {
        calls: calls.clone(),
    }));

    dep.force_stop().await;
    dep.force_stop().await;

    assert_eq!(dep.state(), RunnableState::Faulted);
    assert_eq!(dep.faults().len(), 1);
    assert_eq!(calls.lock().unwrap().force_stopped, 2);
}
