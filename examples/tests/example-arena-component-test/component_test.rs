use arena::{ClosedArena, Component, Dependency, Encounter, EncounterTrait, OpenArena, SetupHandler};
use arena_http::{HttpDependency, ok_json};
use arena_kafka::{KafkaDependency, KafkaFlavor};
use arena_postgres::PostgresDependency;
use arena_executable_component::executable_component::ExecutableComponent;
use arena_examples::http_healthcheck::HttpReadinessCheck;
use async_trait::async_trait;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use rstest::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::sync::OnceCell;

const POSTGRES_PORT: u16 = 5555;
const DB_NAME: &str = "test_database";
const DB_USER: &str = "test_user";
const DB_PASS: &str = "test_password";
const KAFKA_PORT: u16 = 9093;
const HTTP_MOCK_PORT: u16 = 8888;
const EXEC_WEB_APP_PORT: u16 = 3000;

const NETWORK_NAME: &str = "arena-component-test-network";
const POSTGRES_CONTAINER_NAME: &str = "arena-component-test-postgres";
const KAFKA_CONTAINER_NAME: &str = "arena-component-test-kafka";
const HTTP_MOCK_CONTAINER_NAME: &str = "arena-component-test-http-mock";

#[derive(Debug, Serialize, Deserialize)]
struct Reading {
    id: i32,
    user_name: String,
    value: i32,
    comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateReadingResponse {
    valid: bool,
    #[serde(default)]
    id: Option<i64>,
}

#[derive(Debug, Serialize)]
struct CreateReadingRequest {
    user_name: String,
    value: i32,
    comment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReadingCreatedEvent {
    id: i64,
    user_name: String,
    value: i32,
    comment: Option<String>,
}

fn init_logging() {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,arena=debug")
    )
    .is_test(true)
    .try_init();
}

fn setup_dependencies() -> Vec<Dependency> {
    let startup_sql_scripts = vec![
        include_str!("../../resources/instrument_reading_db_schema.sql").to_string()
    ];

    let postgres_db: Dependency = Box::new(
        PostgresDependency::builder("test database")
            .with_image("14.20-trixie")
            .with_port(POSTGRES_PORT)
            .with_database_name(DB_NAME)
            .with_database_username(DB_USER)
            .with_database_password(DB_PASS)
            .with_container_name(POSTGRES_CONTAINER_NAME)
            .with_network(NETWORK_NAME)
            .with_startup_sql_scripts(startup_sql_scripts)
            .build(),
    );

    let kafka: Dependency = Box::new(
        KafkaDependency::builder("test kafka")
            .with_flavor(KafkaFlavor::ApacheNative)
            .with_port(KAFKA_PORT)
            .with_container_name(KAFKA_CONTAINER_NAME)
            .with_network(NETWORK_NAME)
            .with_topic("readings")
            .build(),
    );

    let calibration_service: Dependency = Box::new(
        HttpDependency::builder("calibration service")
            .with_port(HTTP_MOCK_PORT)
            .with_container_name(HTTP_MOCK_CONTAINER_NAME)
            .with_network(NETWORK_NAME)
            .build(),
    );

    vec![postgres_db, kafka, calibration_service]
}

fn resolve_web_app_binary() -> String {
    if let Ok(runfiles) = std::env::var("RUNFILES_DIR") {
        return format!("{runfiles}/_main/examples/web-app");
    }
    "target/release/web-app".to_string()
}

fn setup_exec_component() -> Component {
    let healthcheck_url = format!("http://127.0.0.1:{}/readings", EXEC_WEB_APP_PORT);
    let binary = resolve_web_app_binary();
    let is_bazel = std::env::var("RUNFILES_DIR").is_ok();

    let mut builder = ExecutableComponent::builder("test web app (exec)")
        .with_executable_path(binary)
        .with_env_var("RUST_LOG", "info")
        .with_runtime_arg("web_app_port", EXEC_WEB_APP_PORT.to_string())
        .with_runtime_arg(
            "postgres_connection_string",
            format!(
                "host=localhost port={} user={} password={} dbname={}",
                POSTGRES_PORT, DB_USER, DB_PASS, DB_NAME
            )
        )
        .with_runtime_arg("kafka_bootstrap", format!("localhost:{}", KAFKA_PORT))
        .with_runtime_arg("calibration_url", format!("http://127.0.0.1:{}", HTTP_MOCK_PORT))
        .with_readiness_check(HttpReadinessCheck::new(), healthcheck_url);

    if !is_bazel {
        builder = builder
            .with_source_path("examples")
            .with_build_tool(arena_executable_component::BuildTool::Cargo);
    }

    Box::new(builder.build())
}

struct ValidationServiceSetup;

#[async_trait]
impl SetupHandler for ValidationServiceSetup {
    async fn setup(&self, dependencies: &[Dependency]) {
        let http = dependencies
            .iter()
            .find(|d| d.identifier() == "calibration service")
            .and_then(|d| d.as_any().downcast_ref::<HttpDependency>())
            .expect("calibration service should be available");

        http.playbook()
            .post("/api/v1/validate")
            .will_return(ok_json(json!({ "valid": true })))
            .run()
            .await;
    }
}

async fn create_arena() -> OpenArena {
    let dependencies = setup_dependencies();

    let exec_component = setup_exec_component();
    let components: Vec<Component> = vec![exec_component];

    let encounters: Vec<Box<dyn EncounterTrait>> = vec![Box::new(
        Encounter::new("reading lifecycle", dependencies, components)
            .with_dependency_setup_handler(Box::new(ValidationServiceSetup)),
    )];
    let closed_arena = ClosedArena::new("Test Arena".to_string(), encounters);

    closed_arena.open().await
}

static SHARED_ARENA: OnceCell<OpenArena> = OnceCell::const_new();

#[ctor::dtor]
unsafe fn teardown() {
    if let Some(arena) = SHARED_ARENA.get() {
        let rt = tokio::runtime::Runtime::new().expect("create runtime for teardown");
        let _guard = rt.enter();
        std::ptr::drop_in_place(arena as *const OpenArena as *mut OpenArena);
    }
}

#[fixture]
async fn shared_arena() -> &'static OpenArena {
    init_logging();
    SHARED_ARENA.get_or_init(|| async { create_arena().await }).await
}

async fn get_readings(port: u16) -> Vec<Reading> {
    let url = format!("http://127.0.0.1:{}/readings", port);
    let response = reqwest::get(&url)
        .await
        .expect("GET /readings failed to send");

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        panic!("GET /readings failed (HTTP {status}): {body}");
    }

    response.json::<Vec<Reading>>()
        .await
        .expect("GET /readings returned invalid JSON")
}

fn consume_reading_created_event(
    bootstrap: String,
    topic: String,
    id_rx: std::sync::mpsc::Receiver<i64>,
    timeout: Duration,
) -> Result<ReadingCreatedEvent, String> {
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("group.id", format!("component-test-{}", std::process::id()))
        .set("auto.offset.reset", "earliest")
        .create()
        .map_err(|e| format!("create kafka consumer failed: {e}"))?;

    consumer
        .subscribe(&[&topic])
        .map_err(|e| format!("kafka subscribe failed: {e}"))?;

    let expected_id = id_rx.recv().map_err(|_| "id channel closed")?;

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match consumer.poll(Duration::from_millis(100)) {
            None => continue,
            Some(Err(e)) => return Err(format!("kafka consume error: {e}")),
            Some(Ok(msg)) => {
                let payload = match msg.payload() {
                    Some(bytes) => bytes,
                    None => continue,
                };
                let event: ReadingCreatedEvent = serde_json::from_slice(payload)
                    .map_err(|e| format!("parse ReadingCreatedEvent failed: {e}"))?;
                if event.id == expected_id {
                    return Ok(event);
                }
            }
        }
    }
    Err("did not receive expected ReadingCreatedEvent before timeout".to_string())
}

async fn create_reading(port: u16, user_name: &str, value: i32, comment: Option<String>) -> i32 {
    let url = format!("http://127.0.0.1:{}/readings", port);
    let request = CreateReadingRequest {
        user_name: user_name.to_string(),
        value,
        comment,
    };

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .expect("POST /readings failed to send");

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        panic!("POST /readings failed (HTTP {status}): {body}");
    }

    let create_response: CreateReadingResponse = serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("POST /readings returned invalid JSON: {e}; body: {body}"));

    assert!(
        create_response.valid,
        "expected calibration valid=true in response body: {body}"
    );
    create_response
        .id
        .expect("expected id when calibration accepted reading")
        .try_into()
        .expect("reading id fits i32")
}

#[rstest]
#[tokio::test]
async fn example_using_exec_component_creates_reading_consumes_and_gets_reading(
    #[future] shared_arena: &'static OpenArena,
) {
    let arena = shared_arena.await;

    let bootstrap = arena
        .dependency("test kafka")
        .and_then(|d| d.as_any().downcast_ref::<KafkaDependency>())
        .and_then(|k| k.bootstrap_servers())
        .expect("kafka bootstrap should be available")
        .to_string();

    let (id_tx, id_rx) = std::sync::mpsc::channel();
    let consume_handle = tokio::task::spawn_blocking({
        move || consume_reading_created_event(bootstrap, "readings".to_string(), id_rx, Duration::from_secs(5))
    });

    let created_id = create_reading(EXEC_WEB_APP_PORT, "Exec Test User", 42, Some("test comment".to_string())).await;
    id_tx.send(created_id as i64).expect("send created_id to consumer");

    let consumed = consume_handle
        .await
        .expect("consume task join")
        .expect("should consume ReadingCreatedEvent from Kafka");

    assert_eq!(consumed.id, created_id as i64);
    assert_eq!(consumed.user_name, "Exec Test User");
    assert_eq!(consumed.value, 42);
    assert_eq!(consumed.comment.as_deref(), Some("test comment"));

    let readings = get_readings(EXEC_WEB_APP_PORT).await;
    let found = readings
        .iter()
        .find(|r| r.id == created_id)
        .expect("should find newly created reading");

    assert_eq!(found.id, created_id);
    assert_eq!(found.user_name, "Exec Test User");
    assert_eq!(found.value, 42);
    assert_eq!(found.comment.as_deref(), Some("test comment"));
}
