use std::net::{IpAddr, Ipv4Addr};

use arena::{ClosedArena, Component, Dependency, Match, MatchTrait};
use arena_containerized_component::containerized_component::ContainerizedComponent;
use arena_examples::http_healthcheck::HttpReadinessCheck;
use arena_executable_component::executable_component::ExecutableComponent;
use arena_kafka::kafka_dependency::client::{connect_client, partition_client_for};
use arena_kafka::{KafkaDependency, KafkaFlavor, TopicCreator, KAFKA_INTERNAL_DOCKER_PORT};
use arena_mssql::MssqlDependency;
use arena_oauth::OauthDependency;
use arena_postgres::PostgresDependency;
use rskafka::client::partition::OffsetAt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const OAUTH_PORT: u16 = 9443;
const OAUTH_ISSUER_FOR_WEB_CONTAINER: &str = "https://host.docker.internal:9443";

const POSTGRES_PORT: u16 = 4444;
const POSTGRES_DB_NAME: &str = "readings_db";
const POSTGRES_DB_USER: &str = "readings_user";
const POSTGRES_DB_PASS: &str = "readings_password";
const KAFKA_PORT: u16 = 9092;
const KAFKA_TOPIC: &str = "readings";
const WEB_APP_PORT: u16 = 3001;
const MSSQL_PORT: u16 = 1433;
const MSSQL_DB_NAME: &str = "validationDb";
const MSSQL_DB_USER: &str = "sa";
const MSSQL_DB_PASS: &str = "yourStrong(!)Password";

const NETWORK_NAME: &str = "arena-example-network";
const POSTGRES_CONTAINER_NAME: &str = "arena-example-postgres";
const KAFKA_CONTAINER_NAME: &str = "arena-example-kafka";
const MSSQL_CONTAINER_NAME: &str = "arena-example-mssql";

const KAFKA_CREATE_TOPIC_RETRY_WINDOW_MS: u64 = 2000;
const KAFKA_CREATE_TOPIC_RETRY_INTERVAL_MS: u64 = 250;
const KAFKA_CONSUME_FETCH_MIN_BYTES: i32 = 1;
const KAFKA_CONSUME_FETCH_MAX_BYTES: i32 = 1_000_000;
const KAFKA_CONSUME_FETCH_MAX_WAIT_MS: i32 = 250;
const KAFKA_CONSUMER_SHUTDOWN_TIMEOUT_MS: u64 = 500;

async fn setup_arena_components(oauth_ca_pem: &str) -> Vec<Component> {
    let containerfile = include_str!("../example_readings_axum_web_app/web_server/Dockerfile");

    let web_app = ContainerizedComponent::builder("example web app", containerfile)
        .with_build_context(".")
        .with_image_tag("arena-example-readings-axum-web-app")
        .with_port_mapping(WEB_APP_PORT, 3000)
        .with_host_mapping("host.docker.internal:host-gateway")
        .with_env_var("RUST_LOG", "debug")
        .with_env_var("OAUTH_TLS_CA_PEM", oauth_ca_pem)
        .with_runtime_arg("web_app_port", "3000")
        .with_runtime_arg(
            "postgres_connection_string",
            format!(
                "host={} port=5432 user={} password={} dbname={}",
                POSTGRES_CONTAINER_NAME, POSTGRES_DB_USER, POSTGRES_DB_PASS, POSTGRES_DB_NAME
            ),
        )
        .with_runtime_arg(
            "kafka_bootstrap",
            format!("{}:{}", KAFKA_CONTAINER_NAME, KAFKA_INTERNAL_DOCKER_PORT),
        )
        .with_runtime_arg(
            "calibration_url",
            format!("http://{}:8888", NETWORK_NAME),
        )
        .with_runtime_arg(
            "mssql_connection_string",
            format!(
                "Server=tcp:{},1433;Database={};User Id={};Password={};TrustServerCertificate=True;",
                MSSQL_CONTAINER_NAME, MSSQL_DB_NAME, MSSQL_DB_USER, MSSQL_DB_PASS
            ),
        )
        .with_runtime_arg("oauth_issuer_url", OAUTH_ISSUER_FOR_WEB_CONTAINER)
        .with_network(NETWORK_NAME)
        .with_readiness_check(
            HttpReadinessCheck::new(),
            format!("http://localhost:{}/health", WEB_APP_PORT),
        )
        .build()
        .await
        .expect("build example web app container");

    vec![Box::new(web_app)]
}

fn console_executable_oauth_tls_ca_pem() -> String {
    OauthDependency::builder("example console executable tls snapshot")
        .with_ephemeral_server_tls()
        .with_port(0)
        .build()
        .server_tls_certificate_pem()
        .expect("oauth server tls cert")
        .to_string()
}

#[allow(dead_code)]
fn setup_executable_arena_components() -> Vec<Component> {
    let oauth_ca_pem = console_executable_oauth_tls_ca_pem();
    vec![Box::new(
        ExecutableComponent::builder("example web app")
            .with_source_path("examples")
            .with_build_tool(arena_executable_component::BuildTool::Cargo)
            .with_executable_path("target/release/example-readings-axum-web-app")
            .with_env_var("RUST_LOG", "debug")
            .with_env_var("OAUTH_TLS_CA_PEM", oauth_ca_pem)
            .with_runtime_arg("web_app_port", WEB_APP_PORT.to_string())
            .with_runtime_arg(
                "postgres_connection_string",
                format!(
                    "host=localhost port={} user={} password={} dbname={}",
                    POSTGRES_PORT, POSTGRES_DB_USER, POSTGRES_DB_PASS, POSTGRES_DB_NAME
                ),
            )
            .with_runtime_arg("kafka_bootstrap", format!("localhost:{}", KAFKA_PORT))
            .with_runtime_arg("calibration_url", "http://127.0.0.1:8888".to_string())
            .with_runtime_arg(
                "mssql_connection_string",
                format!(
                    "Server=tcp:localhost,{};Database={};User Id={};Password={};TrustServerCertificate=True;",
                    MSSQL_PORT, MSSQL_DB_NAME, MSSQL_DB_USER, MSSQL_DB_PASS
                ),
            )
            .with_runtime_arg("oauth_issuer_url", format!("https://127.0.0.1:{}", OAUTH_PORT))
            .build(),
    )]
}

fn setup_arena_dependencies() -> (Vec<Dependency>, String, String) {
    let startup_sql_scripts =
        vec![include_str!("../../resources/instrument_reading_db_schema.sql").to_string()];

    let postgres_db = PostgresDependency::builder("example readings")
        .with_image("14.20-trixie")
        .with_port(POSTGRES_PORT)
        .with_database_name(POSTGRES_DB_NAME)
        .with_database_username(POSTGRES_DB_USER)
        .with_database_password(POSTGRES_DB_PASS)
        .with_container_name(POSTGRES_CONTAINER_NAME)
        .with_network(NETWORK_NAME)
        .with_startup_sql_scripts(startup_sql_scripts)
        .build();

    let kafka = KafkaDependency::builder("example readings")
        .with_flavor(KafkaFlavor::ApacheNative)
        .with_port(KAFKA_PORT)
        .with_container_name(KAFKA_CONTAINER_NAME)
        .with_network(NETWORK_NAME)
        .build();
    let kafka_id = kafka.identifier.clone();

    let mssql_startup_sql_scripts =
        vec![include_str!("../../resources/validation_db_schema.sql").to_string()];

    let mssql = MssqlDependency::builder("example validation")
        .with_port(MSSQL_PORT)
        .with_database_name(MSSQL_DB_NAME)
        .with_database_username(MSSQL_DB_USER)
        .with_database_password(MSSQL_DB_PASS)
        .with_container_name(MSSQL_CONTAINER_NAME)
        .with_network(NETWORK_NAME)
        .with_startup_sql_scripts(mssql_startup_sql_scripts)
        .build();

    let oauth = OauthDependency::builder("example oauth")
        .with_ephemeral_server_tls()
        .with_listen_ip(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
        .with_port(OAUTH_PORT)
        .build();
    let oauth_ca_pem = oauth
        .server_tls_certificate_pem()
        .expect("oauth server tls cert")
        .to_string();

    let deps: Vec<Dependency> = vec![
        Box::new(postgres_db),
        Box::new(kafka),
        Box::new(mssql),
        Box::new(oauth),
    ];
    (deps, kafka_id, oauth_ca_pem)
}

async fn create_kafka_topic(bootstrap: &str, topic: &str) {
    create_topic_with_retry(
        bootstrap,
        topic,
        Duration::from_millis(KAFKA_CREATE_TOPIC_RETRY_WINDOW_MS),
    )
    .await;
}

async fn create_topic_with_retry(bootstrap: &str, topic: &str, timeout: Duration) {
    let start = Instant::now();
    let poll_every = Duration::from_millis(KAFKA_CREATE_TOPIC_RETRY_INTERVAL_MS);

    loop {
        if start.elapsed() >= timeout {
            panic!("kafka topic create timed out (topic={topic})");
        }

        match TopicCreator::create_topic_on(bootstrap, topic).await {
            Ok(()) => return,
            Err(err) => {
                tracing::debug!(error = %err, phase = "kafka_topic_create", "topic create failed (will retry)");
            }
        }

        tokio::time::sleep(poll_every).await;
    }
}

struct KafkaConsumerHandle {
    shutdown_signal: Arc<AtomicBool>,
    task: tokio::task::JoinHandle<()>,
}

impl KafkaConsumerHandle {
    async fn shutdown(self) {
        tracing::debug!(phase = "kafka_consumer_shutdown", "signaling kafka consumer shutdown");
        self.shutdown_signal.store(true, Ordering::Relaxed);
        if tokio::time::timeout(
            Duration::from_millis(KAFKA_CONSUMER_SHUTDOWN_TIMEOUT_MS),
            self.task,
        )
        .await
        .is_err()
        {
            tracing::debug!(phase = "kafka_consumer_shutdown", "consumer task did not stop in time");
        }
    }
}

async fn create_output_kafka_consumer(kafka_bootstrap: &str, topic: &str) -> KafkaConsumerHandle {
    let client = connect_client(kafka_bootstrap)
        .await
        .expect("create kafka client");
    let partition = partition_client_for(&client, topic)
        .await
        .expect("create kafka partition client");

    let topic = topic.to_string();
    let should_shutdown = Arc::new(AtomicBool::new(false));
    let should_shutdown_clone = should_shutdown.clone();

    let task = tokio::spawn(async move {
        let mut next_offset = match partition.get_offset(OffsetAt::Earliest).await {
            Ok(v) => v,
            Err(err) => {
                tracing::debug!(error = %err, phase = "kafka_consume_offset", "get kafka earliest offset failed");
                return;
            }
        };

        while !should_shutdown_clone.load(Ordering::Relaxed) {
            match partition
                .fetch_records(
                    next_offset,
                    KAFKA_CONSUME_FETCH_MIN_BYTES..KAFKA_CONSUME_FETCH_MAX_BYTES,
                    KAFKA_CONSUME_FETCH_MAX_WAIT_MS,
                )
                .await
            {
                Err(err) => {
                    tracing::debug!(error = %err, phase = "kafka_consume_poll", "consumer poll returned error");
                }
                Ok((records, _high_watermark)) => {
                    for r in &records {
                        let payload = r
                            .record
                            .value
                            .as_deref()
                            .map(String::from_utf8_lossy)
                            .unwrap_or_default();
                        tracing::debug!(
                            topic = %topic,
                            payload = %payload,
                            phase = "kafka_consume_received",
                            "consumer received message",
                        );
                    }
                    if let Some(last) = records.last() {
                        next_offset = last.offset + 1;
                    }
                }
            }
        }
        tracing::debug!(phase = "kafka_consumer_stopped", "kafka consumer shutting down");
    });

    KafkaConsumerHandle {
        shutdown_signal: should_shutdown,
        task,
    }
}

#[tokio::main]
async fn main() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let (dependencies, kafka_id, oauth_ca_pem) = setup_arena_dependencies();
    let components = setup_arena_components(&oauth_ca_pem).await;
    let matches: Vec<Box<dyn MatchTrait>> = vec![Box::new(Match::new(
        "End to end happy path match",
        dependencies,
        components,
    ))];
    let closed_arena = ClosedArena::new(String::from("Example Arena"), matches);

    let open_arena = closed_arena.open().await;

    let kafka_bootstrap = open_arena
        .dependency(&kafka_id)
        .and_then(|d| d.as_any().downcast_ref::<KafkaDependency>())
        .and_then(|k| k.bootstrap_servers())
        .expect("kafka dependency bootstrap should be available after open()")
        .to_string();

    create_kafka_topic(&kafka_bootstrap, KAFKA_TOPIC).await;
    let consumer_handle = create_output_kafka_consumer(&kafka_bootstrap, KAFKA_TOPIC).await;

    tokio::signal::ctrl_c().await.unwrap();

    consumer_handle.shutdown().await;
    drop(open_arena);
}
