use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_postgres::{PostgresDependency, PostgresImpl};
use async_trait::async_trait;
use futures::FutureExt;
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

struct PanickingPostgresReadinessCheck;

#[async_trait]
impl ReadinessCheck for PanickingPostgresReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _connection_string: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        panic!("readiness probe failed");
    }
}

fn postgres_stop_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::PostgresStop))
        .count()
}

fn build_postgres(events: Arc<Mutex<Vec<Event>>>) -> PostgresDependency {
    PostgresDependency::builder("postgres-drop")
        .with_impl(FakePostgresImpl {
            conn_str: None,
            events,
        })
        .with_readiness_check(OkReadinessCheck)
        .build()
}

#[test]
fn drop_unstarted_dep_skips_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let dep = build_postgres(events.clone());
    drop(dep);
    assert_eq!(postgres_stop_count(&events.lock().unwrap()), 0);
}

#[tokio::test]
async fn stop_then_drop_single_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_postgres(events.clone());
    dep.start().await;
    dep.stop().await;
    drop(dep);
    assert_eq!(postgres_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn drop_running_dep_invokes_full_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_postgres(events.clone());
    dep.start().await;
    drop(dep);
    assert_eq!(postgres_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn start_panic_then_drop_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = PostgresDependency::builder("postgres-drop")
        .with_impl(FakePostgresImpl {
            conn_str: None,
            events: events.clone(),
        })
        .with_readiness_check(PanickingPostgresReadinessCheck)
        .build();

    let start_outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await;
    })
    .catch_unwind()
    .await;
    assert!(start_outcome.is_err());
    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[Event::PostgresStart]
    );

    drop(dep);
    assert_eq!(postgres_stop_count(&events.lock().unwrap()), 1);
}
