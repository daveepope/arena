use arena::{ClosedArena, Component, Dependency, Match, MatchTrait, OpenArena};
use arena_examples::example_axum_web_server::state::build_http_client_trusting_oauth_ca;
use arena_examples::http_healthcheck::HttpReadinessCheck;
use arena_executable_component::executable_component::ExecutableComponent;
use arena_http::{ok_json, server_error, HttpDependency, ManagedHttpPlaybook};
use arena_kafka::{KafkaDependency, KafkaFlavor};
use arena_mssql::MssqlDependency;
use arena_oauth::OauthDependency;
use arena_postgres::PostgresDependency;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;

static KAFKA_ID: OnceLock<String> = OnceLock::new();
static CALIBRATION_ID: OnceLock<String> = OnceLock::new();
static MSSQL_ID: OnceLock<String> = OnceLock::new();
static OAUTH_ID: OnceLock<String> = OnceLock::new();

const OAUTH_PORT: u16 = 9443;
const OAUTH_ISSUER: &str = "https://127.0.0.1:9443";
const OAUTH_TLS_CERT_PEM: &str = include_str!("../../resources/oauth_tls_cert.pem");
const OAUTH_TLS_KEY_PEM: &str = include_str!("../../resources/oauth_tls_key.pem");

const POSTGRES_PORT: u16 = 5555;
const POSTGRES_DB_NAME: &str = "readings_db";
const POSTGRES_DB_USER: &str = "readings_user";
const POSTGRES_DB_PASS: &str = "readings_password";
const KAFKA_PORT: u16 = 9093;
const CALIBRATION_HTTP_PORT: u16 = 8888;
const EXEC_WEB_APP_PORT: u16 = 3000;
const MSSQL_PORT: u16 = 1435;
const MSSQL_DB_NAME: &str = "validationDb";
const MSSQL_DB_USER: &str = "sa";
const MSSQL_DB_PASS: &str = "yourStrong(!)Password";

const NETWORK_NAME: &str = "arena-component-test-network";

async fn fetch_example_access_token_with_scope(scope: Option<&str>) -> String {
    let client = build_http_client_trusting_oauth_ca(OAUTH_TLS_CERT_PEM);
    arena_examples::oauth_client_credentials::fetch_client_credentials_access_token(
        &client,
        OAUTH_ISSUER,
        scope,
    )
    .await
    .expect("fetch client_credentials access token")
}

async fn fetch_example_access_token() -> String {
    fetch_example_access_token_with_scope(Some("openid profile readings"))
        .await
}

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
        env_logger::Env::default().default_filter_or("info,arena=debug"),
    )
    .is_test(true)
    .try_init();
}

fn setup_dependencies() -> Vec<Dependency> {
    let startup_sql_scripts =
        vec![include_str!("../../resources/instrument_reading_db_schema.sql").to_string()];

    let postgres_db = PostgresDependency::builder("example readings")
        .with_image("14.20-trixie")
        .with_port(POSTGRES_PORT)
        .with_database_name(POSTGRES_DB_NAME)
        .with_database_username(POSTGRES_DB_USER)
        .with_database_password(POSTGRES_DB_PASS)
        .with_network(NETWORK_NAME)
        .with_startup_sql_scripts(startup_sql_scripts)
        .build();

    let kafka = KafkaDependency::builder("example readings")
        .with_flavor(KafkaFlavor::ApacheNative)
        .with_port(KAFKA_PORT)
        .with_network(NETWORK_NAME)
        .with_topic("readings")
        .build();
    KAFKA_ID
        .set(kafka.identifier.clone())
        .expect("kafka id set once");

    let calibration_service = HttpDependency::builder("example calibration")
        .with_port(CALIBRATION_HTTP_PORT)
        .with_network(NETWORK_NAME)
        .build();
    CALIBRATION_ID
        .set(calibration_service.identifier.clone())
        .expect("calibration id set once");

    let mssql_startup_sql_scripts =
        vec![include_str!("../../resources/validation_db_schema.sql").to_string()];

    let validation_db = MssqlDependency::builder("example validation")
        .with_port(MSSQL_PORT)
        .with_database_name(MSSQL_DB_NAME)
        .with_database_username(MSSQL_DB_USER)
        .with_database_password(MSSQL_DB_PASS)
        .with_network(NETWORK_NAME)
        .with_startup_sql_scripts(mssql_startup_sql_scripts)
        .build();
    MSSQL_ID
        .set(validation_db.identifier.clone())
        .expect("mssql id set once");

    let oauth = OauthDependency::builder("component test oauth")
        .with_server_tls_pem(OAUTH_TLS_CERT_PEM, OAUTH_TLS_KEY_PEM)
        .with_listen_ip(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
        .with_port(OAUTH_PORT)
        .build();
    OAUTH_ID
        .set(oauth.identifier.clone())
        .expect("oauth id set once");

    vec![
        Box::new(postgres_db),
        Box::new(kafka),
        Box::new(calibration_service),
        Box::new(validation_db),
        Box::new(oauth),
    ]
}

fn resolve_web_app_binary() -> String {
    if let Ok(runfiles) = std::env::var("RUNFILES_DIR") {
        return format!("{runfiles}/_main/examples/web-app");
    }
    "target/release/web-app".to_string()
}

fn setup_exec_component() -> Component {
    let healthcheck_url = format!("http://127.0.0.1:{}/health", EXEC_WEB_APP_PORT);
    let binary = resolve_web_app_binary();
    let is_bazel = std::env::var("RUNFILES_DIR").is_ok();

    let mut builder = ExecutableComponent::builder("example web app")
        .with_executable_path(binary)
        .with_env_var("RUST_LOG", "info")
        .with_env_var("OAUTH_TLS_CA_PEM", OAUTH_TLS_CERT_PEM)
        .with_env_var("OAUTH_REQUIRED_ACCESS_TOKEN_SCOPES", "readings")
        .with_runtime_arg("web_app_port", EXEC_WEB_APP_PORT.to_string())
        .with_runtime_arg(
            "postgres_connection_string",
            format!(
                "host=localhost port={} user={} password={} dbname={}",
                POSTGRES_PORT, POSTGRES_DB_USER, POSTGRES_DB_PASS, POSTGRES_DB_NAME
            ),
        )
        .with_runtime_arg("kafka_bootstrap", format!("localhost:{}", KAFKA_PORT))
        .with_runtime_arg("calibration_url", format!("http://127.0.0.1:{}", CALIBRATION_HTTP_PORT))
        .with_runtime_arg(
            "mssql_connection_string",
            format!(
                "Server=tcp:localhost,{};Database={};User Id={};Password={};TrustServerCertificate=True;",
                MSSQL_PORT, MSSQL_DB_NAME, MSSQL_DB_USER, MSSQL_DB_PASS
            ),
        )
        .with_runtime_arg("oauth_issuer_url", OAUTH_ISSUER)
        .with_readiness_check(HttpReadinessCheck::new(), healthcheck_url);

    if !is_bazel {
        builder = builder
            .with_source_path("examples")
            .with_build_tool(arena_executable_component::BuildTool::Cargo);
    }

    Box::new(builder.build())
}

fn calibration_default_playbook() -> ManagedHttpPlaybook {
    let calibration_id = CALIBRATION_ID
        .get()
        .expect("calibration id initialized")
        .to_string();
    ManagedHttpPlaybook::new("calibration-default", calibration_id, |pb| {
        pb.post("/api/v1/validate")
            .will_return(ok_json(json!({ "valid": true })))
            .into_playbook()
    })
}

async fn create_arena() -> OpenArena {
    let dependencies = setup_dependencies();

    let exec_component = setup_exec_component();
    let components: Vec<Component> = vec![exec_component];

    let matches: Vec<Box<dyn MatchTrait>> = vec![Box::new(
        Match::new("reading lifecycle", dependencies, components)
            .register_playbook(Box::new(calibration_default_playbook()), true),
    )];
    let closed_arena = ClosedArena::new("Test Arena".to_string(), matches);

    closed_arena.open().await
}

static SHARED_ARENA: OnceCell<OpenArena> = OnceCell::const_new();

static SCENARIO_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

static COMPONENT_TEST_RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn component_test_runtime() -> &'static Runtime {
    COMPONENT_TEST_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("component test tokio runtime")
    })
}

async fn shared_arena() -> &'static OpenArena {
    init_logging();
    SHARED_ARENA
        .get_or_init(|| async { create_arena().await })
        .await
}

#[ctor::dtor]
unsafe fn teardown() {
    if let Some(arena) = SHARED_ARENA.get() {
        let _guard = component_test_runtime().enter();
        std::ptr::drop_in_place(arena as *const OpenArena as *mut OpenArena);
    }
}

async fn get_readings(port: u16) -> Vec<Reading> {
    let token = fetch_example_access_token().await;
    let url = format!("http://127.0.0.1:{}/readings", port);
    let response = build_http_client_trusting_oauth_ca(OAUTH_TLS_CERT_PEM)
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .expect("GET /readings failed to send");

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        panic!("GET /readings failed (HTTP {status}): {body}");
    }

    response
        .json::<Vec<Reading>>()
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
    let token = fetch_example_access_token().await;
    let url = format!("http://127.0.0.1:{}/readings", port);
    let request = CreateReadingRequest {
        user_name: user_name.to_string(),
        value,
        comment,
    };

    let client = build_http_client_trusting_oauth_ca(OAUTH_TLS_CERT_PEM);
    let response = client
        .post(&url)
        .bearer_auth(token)
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

#[test]
fn example_using_exec_component_creates_reading_consumes_and_gets_reading() {
    component_test_runtime().block_on(async {
    let arena = shared_arena().await;
    let _scenario = SCENARIO_LOCK.lock().await;

    let _oauth = arena
        .dependency(OAUTH_ID.get().expect("oauth id initialized"))
        .and_then(|d| d.as_any().downcast_ref::<OauthDependency>())
        .expect("oauth dependency");
    assert!(
        _oauth.base_url().is_some(),
        "oauth base_url should be set after arena open"
    );

    let bootstrap = arena
        .dependency(KAFKA_ID.get().expect("kafka id initialized"))
        .and_then(|d| d.as_any().downcast_ref::<KafkaDependency>())
        .and_then(|k| k.bootstrap_servers())
        .expect("kafka bootstrap should be available")
        .to_string();

    let validation_db = arena
        .dependency(MSSQL_ID.get().expect("mssql id initialized"))
        .and_then(|d| d.as_any().downcast_ref::<MssqlDependency>())
        .expect("validation database should be available");
    let validation_playbook = validation_db.playbook().run().await;

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

    let validation_count = validation_playbook
        .verify("SELECT COUNT(*) FROM dbo.validation_results WHERE user_name = N'Exec Test User' AND value = 42 AND valid = 1;")
        .await;
    assert_eq!(
        validation_count, 1,
        "web app should have written one valid validation row to mssql"
    );
    });
}

#[test]
fn example_using_exec_component_calibration_outage_returns_error() {
    component_test_runtime().block_on(async {
        let arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;

        let calibration = arena
            .dependency(CALIBRATION_ID.get().expect("calibration id initialized"))
            .and_then(|d| d.as_any().downcast_ref::<HttpDependency>())
            .expect("calibration service should be available");

        {
            let _outage = calibration
                .playbook()
                .post("/api/v1/validate")
                .with_priority(1)
                .will_return(server_error())
                .run()
                .await;

            let token = fetch_example_access_token().await;
            let url = format!("http://127.0.0.1:{}/readings", EXEC_WEB_APP_PORT);
            let client = build_http_client_trusting_oauth_ca(OAUTH_TLS_CERT_PEM);
            let response = client
                .post(&url)
                .bearer_auth(token)
                .json(&CreateReadingRequest {
                    user_name: "Outage Test User".to_string(),
                    value: 99,
                    comment: None,
                })
                .send()
                .await
                .expect("POST /readings failed to send");

            assert_eq!(
                response.status().as_u16(),
                500,
                "expected 500 while calibration is in outage playbook",
            );
        }

        let recovered_id = create_reading(
            EXEC_WEB_APP_PORT,
            "Recovery Test User",
            17,
            Some("post-outage".to_string()),
        )
        .await;

        let readings = get_readings(EXEC_WEB_APP_PORT).await;
        let found = readings
            .iter()
            .find(|r| r.id == recovered_id)
            .expect("recovered reading should be present");
        assert_eq!(found.user_name, "Recovery Test User");
        assert_eq!(found.value, 17);
    });
}

#[test]
fn example_using_exec_component_readings_returns_401_when_access_token_scopes_insufficient() {
    component_test_runtime().block_on(async {
        let _arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;

        let token = fetch_example_access_token_with_scope(Some("openid profile"))
            .await;
        let url = format!("http://127.0.0.1:{}/readings", EXEC_WEB_APP_PORT);
        let client = build_http_client_trusting_oauth_ca(OAUTH_TLS_CERT_PEM);
        let response = client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .expect("GET /readings send");

        assert_eq!(
            response.status().as_u16(),
            401,
            "GET /readings without required scope should be rejected"
        );
    });
}

#[test]
fn example_using_exec_component_readings_returns_401_when_bearer_token_invalid() {
    component_test_runtime().block_on(async {
        let _arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;

        let url = format!("http://127.0.0.1:{}/readings", EXEC_WEB_APP_PORT);
        let client = build_http_client_trusting_oauth_ca(OAUTH_TLS_CERT_PEM);
        let response = client
            .get(&url)
            .header(
                reqwest::header::AUTHORIZATION,
                "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.signature_not_from_issuer",
            )
            .send()
            .await
            .expect("GET /readings send");

        assert_eq!(
            response.status().as_u16(),
            401,
            "GET /readings with invalid JWT should be rejected"
        );
    });
}
