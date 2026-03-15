use arena::{ClosedArena, Component, Dependency, Encounter, EncounterTrait, OpenArena};
use arena_kafka::{KafkaDependency, KafkaFlavor};
use arena_postgres::PostgresDependency;
use arena_executable_component::executable_component::ExecutableComponent;
use arena_examples::http_healthcheck::HttpReadinessCheck;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::config::ClientConfig;
use rstest::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::OnceCell;

const POSTGRES_PORT: u16 = 5555;
const DB_NAME: &str = "test_database";
const DB_USER: &str = "test_user";
const DB_PASS: &str = "test_password";
const KAFKA_PORT: u16 = 9093;
const KAFKA_TOPIC: &str = "test_readings";
const EXEC_WEB_APP_PORT: u16 = 3000;

const NETWORK_NAME: &str = "arena-component-test-network";
const POSTGRES_CONTAINER_NAME: &str = "arena-component-test-postgres";
const KAFKA_CONTAINER_NAME: &str = "arena-component-test-kafka";

#[derive(Debug, Serialize, Deserialize)]
struct Reading {
    id: i32,
    user_name: String,
    value: i32,
    comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateReadingResponse {
    id: i32,
}

#[derive(Debug, Serialize)]
struct CreateReadingRequest {
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
            .build(),
    );

    vec![postgres_db, kafka]
}

fn setup_exec_component() -> Component {
    let healthcheck_url = format!("http://127.0.0.1:{}/readings", EXEC_WEB_APP_PORT);

    Box::new(
        ExecutableComponent::builder("test web app (exec)")
            .with_source_path("examples")
            .with_build_tool(arena_executable_component::BuildTool::Cargo)
            .with_executable_path("target/release/web-app")
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
            .with_readiness_check(HttpReadinessCheck::new(), healthcheck_url)
            .build()
    )
}

async fn setup_kafka_topic(arena: &OpenArena) {
    let kafka_bootstrap = arena
        .dependency("test kafka")
        .and_then(|d| d.as_any().downcast_ref::<KafkaDependency>())
        .and_then(|k| k.bootstrap_servers())
        .expect("kafka bootstrap should be available");

    let admin: AdminClient<_> = ClientConfig::new()
        .set("bootstrap.servers", kafka_bootstrap)
        .create()
        .expect("create kafka admin client");

    let new_topic = NewTopic::new(KAFKA_TOPIC, 1, TopicReplication::Fixed(1));
    let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(5)));

    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(30) {
            panic!("kafka topic create timed out");
        }

        match admin.create_topics([&new_topic], &opts).await {
            Ok(results) => {
                for r in results {
                    if let Err((_t, e)) = r {
                        if !e.to_string().to_lowercase().contains("already exists") {
                            continue;
                        }
                    }
                }
                return;
            }
            Err(_) => {}
        }

        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn create_arena() -> OpenArena {
    let dependencies = setup_dependencies();

    let exec_component = setup_exec_component();
    let components: Vec<Component> = vec![exec_component];

    let encounters: Vec<Box<dyn EncounterTrait>> = vec![
        Box::new(Encounter::new("reading lifecycle", dependencies, components))
    ];
    let closed_arena = ClosedArena::new("Test Arena".to_string(), encounters);

    let arena = closed_arena.open().await;
    setup_kafka_topic(&arena).await;
    arena
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
    SHARED_ARENA.get_or_init(|| async {
        create_arena().await
    }).await
}

async fn get_readings(port: u16) -> Vec<Reading> {
    let url = format!("http://127.0.0.1:{}/readings", port);
    let response = reqwest::get(&url)
        .await
        .expect("get readings request");
    
    response.json::<Vec<Reading>>()
        .await
        .expect("parse readings response")
}

async fn create_reading(port: u16, user_name: &str, value: i32, comment: Option<String>) -> i32 {
    let url = format!("http://127.0.0.1:{}/readings", port);
    let request = CreateReadingRequest {
        user_name: user_name.to_string(),
        value,
        comment,
    };
    
    let client = reqwest::Client::new();
    let response = client.post(&url)
        .json(&request)
        .send()
        .await
        .expect("create reading request");
    
    let create_response = response.json::<CreateReadingResponse>()
        .await
        .expect("parse create reading response");
    
    create_response.id
}

#[rstest]
#[tokio::test]
async fn example_using_exec_component_creates_reading_consumes_and_gets_reading(
    #[future] shared_arena: &'static OpenArena,
) {
    let _arena = shared_arena.await;

    let created_id = create_reading(EXEC_WEB_APP_PORT, "Exec Test User", 42, Some("test comment".to_string())).await;

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
