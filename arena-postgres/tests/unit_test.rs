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
    ReadinessCheck,
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

#[tokio::test]
async fn postgres_dependency_lifecycle() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let last_identifier = Arc::new(Mutex::new(None::<String>));
    let last_connection_string = Arc::new(Mutex::new(None::<String>));
    let last_timeout_ms = Arc::new(Mutex::new(None::<u64>));

    let mut pg = PostgresDependency::builder("postgres")
        .with_impl(FakePostgresImpl {
            conn_str: None,
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
        pg.start().await;
        pg.stop().await;
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_ok(), "expected start/stop not to panic");

    let got = events.lock().unwrap().clone();
    assert_eq!(
        got,
        vec![
            Event::PostgresStart,
            Event::ReadinessCheck,
            Event::PostgresStop
        ]
    );

    assert_eq!(
        last_identifier.lock().unwrap().as_deref(),
        Some(pg.identifier.as_str())
    );
    assert_eq!(
        last_connection_string.lock().unwrap().as_deref(),
        Some("postgres://127.0.0.1:5432/fake")
    );
    assert_eq!(*last_timeout_ms.lock().unwrap(), Some(10_000));
}
