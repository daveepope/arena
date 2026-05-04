use std::net::{IpAddr, Ipv4Addr};

use arena::{ClosedArena, Component, Dependency, Match, MatchTrait};
use arena_containerized_component::containerized_component::ContainerizedComponent;
use arena_examples::http_healthcheck::HttpReadinessCheck;
use arena_executable_component::executable_component::ExecutableComponent;
use arena_kafka::{KafkaDependency, KafkaFlavor, KAFKA_INTERNAL_DOCKER_PORT};
use arena_mssql::MssqlDependency;
use arena_oauth::OauthDependency;
use arena_postgres::PostgresDependency;
use env_logger::Env;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const OAUTH_TLS_CERT_PEM: &str = include_str!("../../resources/oauth_tls_cert.pem");
const OAUTH_TLS_KEY_PEM: &str = include_str!("../../resources/oauth_tls_key.pem");
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

async fn setup_arena_components() -> Vec<Component> {
    let containerfile = include_str!("../example_readings_axum_web_app/web_server/Dockerfile");

    let web_app = ContainerizedComponent::builder("example web app", containerfile)
        .with_build_context(".")
        .with_image_tag("arena-example-readings-axum-web-app")
        .with_port_mapping(WEB_APP_PORT, 3000)
        .with_host_mapping("host.docker.internal:host-gateway")
        .with_env_var("RUST_LOG", "debug")
        .with_env_var("OAUTH_TLS_CA_PEM", OAUTH_TLS_CERT_PEM)
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
        .await;

    vec![Box::new(web_app)]
}

#[allow(dead_code)]
fn setup_executable_arena_components() -> Vec<Component> {
    vec![Box::new(
        ExecutableComponent::builder("example web app")
            .with_source_path("examples")
            .with_build_tool(arena_executable_component::BuildTool::Cargo)
            .with_executable_path("target/release/example-readings-axum-web-app")
            .with_env_var("RUST_LOG", "debug")
            .with_env_var("OAUTH_TLS_CA_PEM", OAUTH_TLS_CERT_PEM)
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

fn setup_arena_dependencies() -> (Vec<Dependency>, String) {
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
        .with_server_tls_pem(OAUTH_TLS_CERT_PEM, OAUTH_TLS_KEY_PEM)
        .with_listen_ip(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
        .with_port(OAUTH_PORT)
        .build();

    let deps: Vec<Dependency> = vec![
        Box::new(postgres_db),
        Box::new(kafka),
        Box::new(mssql),
        Box::new(oauth),
    ];
    (deps, kafka_id)
}

async fn create_kafka_topic(bootstrap: &str, topic: &str) {
    create_topic_with_retry(bootstrap, topic, Duration::from_secs(10)).await;
}

async fn create_topic_with_retry(bootstrap: &str, topic: &str, timeout: Duration) {
    let start = Instant::now();
    let poll_every = Duration::from_millis(250);

    loop {
        if start.elapsed() >= timeout {
            panic!("kafka topic create timed out (topic={topic})");
        }

        let admin: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .create()
            .expect("create kafka admin client");

        let new_topic = NewTopic::new(topic, 1, TopicReplication::Fixed(1));
        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(2)));

        let ok = match admin.create_topics([&new_topic], &opts).await {
            Ok(results) => {
                let mut ok = true;
                for r in results {
                    if let Err((_t, e)) = r {
                        if e.to_string().to_lowercase().contains("already exists") {
                            ok = true;
                            break;
                        }
                        ok = false;
                        log::debug!("kafka topic create failed: {e}");
                        break;
                    }
                }
                ok
            }
            Err(err) => {
                log::debug!("kafka topic create request failed: {err}");
                false
            }
        };

        if ok {
            return;
        }

        tokio::time::sleep(poll_every).await;
    }
}

struct KafkaConsumerHandle {
    shutdown_signal: Arc<AtomicBool>,
}

impl Drop for KafkaConsumerHandle {
    fn drop(&mut self) {
        log::debug!("dropping kafka consumer handle, signaling shutdown");
        self.shutdown_signal.store(true, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn create_output_kafka_consumer(kafka_bootstrap: &str, topic: &str) -> KafkaConsumerHandle {
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", kafka_bootstrap)
        .set("group.id", "example-console")
        .set("enable.partition.eof", "false")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("create kafka consumer");

    consumer.subscribe(&[topic]).expect("subscribe");

    let topic = topic.to_string();
    let should_shutdown = Arc::new(AtomicBool::new(false));
    let should_shutdown_clone = should_shutdown.clone();

    tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            while !should_shutdown_clone.load(Ordering::Relaxed) {
                match consumer.poll(Duration::from_millis(250)) {
                    None => {}
                    Some(Err(err)) => {
                        log::debug!("kafka consume error: {err}");
                    }
                    Some(Ok(msg)) => {
                        let payload = msg.payload_view::<str>().and_then(|r| r.ok()).unwrap_or("");
                        log::debug!("kafka received {}: {}", topic, payload);
                    }
                }
            }
            log::debug!("kafka consumer shutting down");
        })
        .await
        .ok();
    });

    KafkaConsumerHandle {
        shutdown_signal: should_shutdown,
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        Env::default().default_filter_or(
            "arena=debug,arena_examples=debug,arena_postgres=debug,arena_kafka=debug,testcontainers=info,testcontainers_modules=info,arena_oauth=debug",
        ),
    )
    .init();

    let (dependencies, kafka_id) = setup_arena_dependencies();
    let components = setup_arena_components().await;
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
    let _consumer_handle = create_output_kafka_consumer(&kafka_bootstrap, KAFKA_TOPIC);

    tokio::signal::ctrl_c().await.unwrap();

    drop(open_arena);
}
