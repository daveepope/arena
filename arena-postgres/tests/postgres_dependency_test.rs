use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_postgres::{PostgresDependency, PostgresImpl};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    PostgresStart,
    PostgresStop,
}

struct FakePostgresImpl {
    conn_str: Option<String>,
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
    ) {
        self.conn_str = Some("postgres://127.0.0.1:5432/fake".to_string());
        self.events.lock().unwrap().push(Event::PostgresStart);
    }

    async fn stop(&mut self) {
        self.conn_str = None;
        self.events.lock().unwrap().push(Event::PostgresStop);
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
    PostgresDependency::builder("postgres-reset")
        .with_impl(FakePostgresImpl {
            conn_str: None,
            events,
        })
        .with_readiness_check(AlwaysOkReadinessCheck)
        .build()
}

#[tokio::test]
async fn soft_reset_not_running_returns_early() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let dep = build_dep(events.clone());

    dep.soft_reset().await;

    assert!(events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn soft_reset_running_no_scripts_returns_early() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_dep(events.clone());
    dep.start().await;

    dep.soft_reset().await;

    dep.stop().await;
}

#[tokio::test]
async fn hard_reset_not_running_returns_early() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_dep(events.clone());

    dep.hard_reset().await;

    assert!(events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn hard_reset_running_restarts_container() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_dep(events.clone());
    dep.start().await;

    dep.hard_reset().await;

    let got = events.lock().unwrap().clone();
    assert_eq!(
        got,
        vec![
            Event::PostgresStart,
            Event::PostgresStop,
            Event::PostgresStart,
        ]
    );

    dep.stop().await;
}

#[tokio::test]
async fn drop_needs_teardown_without_running_invokes_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_dep(events.clone());
    dep.start().await;
    dep.hard_reset().await;
    dep.stop().await;

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
