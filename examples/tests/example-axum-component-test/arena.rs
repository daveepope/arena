use arena::{ClosedArena, Component, Dependency, Match, MatchTrait, OpenArena};
use arena_examples::http_healthcheck::HttpReadinessCheck;
use arena_executable_component::executable_component::ExecutableComponent;
use arena_http::HttpDependency;
use arena_kafka::{KafkaDependency, KafkaFlavor};
use arena_mssql::MssqlDependency;
use arena_oauth::OauthDependency;
use arena_postgres::PostgresDependency;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;

pub static KAFKA_ID: OnceLock<String> = OnceLock::new();
pub static CALIBRATION_ID: OnceLock<String> = OnceLock::new();
pub static MSSQL_ID: OnceLock<String> = OnceLock::new();
pub static OAUTH_ID: OnceLock<String> = OnceLock::new();

static OAUTH_SERVER_TLS_CERT_PEM: OnceLock<String> = OnceLock::new();

pub struct TestRuntime {
    pub oauth_port: u16,
    pub oauth_issuer: String,
    pub postgres_port: u16,
    pub kafka_port: u16,
    pub calibration_http_port: u16,
    pub exec_web_app_port: u16,
    pub mssql_port: u16,
    pub network_name: String,
}

fn ephemeral_tcp_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral tcp port")
        .local_addr()
        .expect("local_addr")
        .port()
}

fn run_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .subsec_nanos();
    format!("{:x}-{:x}", std::process::id(), nanos)
}

static TEST_RUNTIME: OnceLock<TestRuntime> = OnceLock::new();

pub fn test_runtime() -> &'static TestRuntime {
    TEST_RUNTIME.get_or_init(|| {
        let oauth_port = ephemeral_tcp_port();
        TestRuntime {
            oauth_issuer: format!("https://127.0.0.1:{oauth_port}"),
            oauth_port,
            postgres_port: ephemeral_tcp_port(),
            kafka_port: ephemeral_tcp_port(),
            calibration_http_port: ephemeral_tcp_port(),
            exec_web_app_port: ephemeral_tcp_port(),
            mssql_port: ephemeral_tcp_port(),
            network_name: format!("arena-example-api-network-{}", run_suffix()),
        }
    })
}

pub fn oauth_issuer() -> &'static str {
    &test_runtime().oauth_issuer
}

pub fn exec_web_app_port() -> u16 {
    test_runtime().exec_web_app_port
}

pub fn oauth_server_tls_cert_pem() -> &'static str {
    OAUTH_SERVER_TLS_CERT_PEM
        .get()
        .map(|s| s.as_str())
        .expect("oauth_server_tls_cert_pem: arena dependencies not initialized")
}

pub const POSTGRES_DB_NAME: &str = "readings_db";
pub const POSTGRES_DB_USER: &str = "readings_user";
pub const POSTGRES_DB_PASS: &str = "readings_password";
pub const MSSQL_DB_NAME: &str = "validationDb";
pub const MSSQL_DB_USER: &str = "sa";
pub const MSSQL_DB_PASS: &str = "yourStrong(!)Password";

pub fn setup_dependencies() -> Vec<Dependency> {
    let rt = test_runtime();
    let startup_sql_scripts =
        vec![include_str!("../../resources/instrument_reading_db_schema.sql").to_string()];

    let postgres_db = PostgresDependency::builder("example-api-postgres")
        .with_image("14.20-trixie")
        .with_port(rt.postgres_port)
        .with_database_name(POSTGRES_DB_NAME)
        .with_database_username(POSTGRES_DB_USER)
        .with_database_password(POSTGRES_DB_PASS)
        .with_network(&rt.network_name)
        .with_startup_sql_scripts(startup_sql_scripts)
        .build();

    let kafka = KafkaDependency::builder("example-api-kafka")
        .with_flavor(KafkaFlavor::ApacheNative)
        .with_port(rt.kafka_port)
        .with_network(&rt.network_name)
        .with_topic("readings")
        .build();
    KAFKA_ID
        .set(kafka.identifier.clone())
        .expect("kafka id set once");

    let calibration_service = HttpDependency::builder("example-api-calibration")
        .with_port(rt.calibration_http_port)
        .with_network(&rt.network_name)
        .build();
    CALIBRATION_ID
        .set(calibration_service.identifier.clone())
        .expect("calibration id set once");

    let mssql_startup_sql_scripts =
        vec![include_str!("../../resources/validation_db_schema.sql").to_string()];

    let validation_db = MssqlDependency::builder("example-api-mssql")
        .with_port(rt.mssql_port)
        .with_database_name(MSSQL_DB_NAME)
        .with_database_username(MSSQL_DB_USER)
        .with_database_password(MSSQL_DB_PASS)
        .with_network(&rt.network_name)
        .with_startup_sql_scripts(mssql_startup_sql_scripts)
        .build();
    MSSQL_ID
        .set(validation_db.identifier.clone())
        .expect("mssql id set once");

    let oauth = OauthDependency::builder("example-api-oauth")
        .with_ephemeral_server_tls()
        .with_listen_ip(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
        .with_port(rt.oauth_port)
        .with_metadata_base_url(&rt.oauth_issuer)
        .build();
    OAUTH_SERVER_TLS_CERT_PEM
        .set(
            oauth
                .server_tls_certificate_pem()
                .expect("oauth server tls cert")
                .to_string(),
        )
        .expect("oauth server tls cert set once");
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

pub fn resolve_web_app_binary() -> String {
    if let Ok(runfiles) = std::env::var("RUNFILES_DIR") {
        return format!("{runfiles}/_main/examples/example-readings-axum-web-app");
    }
    "target/release/example-readings-axum-web-app".to_string()
}

pub fn setup_exec_component() -> Component {
    let rt = test_runtime();
    let healthcheck_url = format!("http://127.0.0.1:{}/health", rt.exec_web_app_port);
    let binary = resolve_web_app_binary();
    let is_bazel = std::env::var("RUNFILES_DIR").is_ok();

    let mut builder = ExecutableComponent::builder("example-api-web-app")
        .with_executable_path(binary)
        .with_env_var("RUST_LOG", "info")
        .with_env_var("OAUTH_TLS_CA_PEM", oauth_server_tls_cert_pem())
        .with_env_var("OAUTH_REQUIRED_ACCESS_TOKEN_SCOPES", "readings")
        .with_runtime_arg("web_app_port", rt.exec_web_app_port.to_string())
        .with_runtime_arg(
            "postgres_connection_string",
            format!(
                "host=localhost port={} user={} password={} dbname={}",
                rt.postgres_port, POSTGRES_DB_USER, POSTGRES_DB_PASS, POSTGRES_DB_NAME
            ),
        )
        .with_runtime_arg("kafka_bootstrap", format!("localhost:{}", rt.kafka_port))
        .with_runtime_arg(
            "calibration_url",
            format!("http://127.0.0.1:{}", rt.calibration_http_port),
        )
        .with_runtime_arg(
            "mssql_connection_string",
            format!(
                "Server=tcp:localhost,{};Database={};User Id={};Password={};TrustServerCertificate=True;encrypt=DANGER_PLAINTEXT;",
                rt.mssql_port, MSSQL_DB_NAME, MSSQL_DB_USER, MSSQL_DB_PASS
            ),
        )
        .with_runtime_arg("oauth_issuer_url", rt.oauth_issuer.clone())
        .with_readiness_check(HttpReadinessCheck::new(), healthcheck_url);

    if !is_bazel {
        builder = builder
            .with_source_path("examples")
            .with_build_tool(arena_executable_component::BuildTool::Cargo);
    }

    Box::new(builder.build())
}

use crate::playbooks::{
    calibration_api_error_path_playbook, calibration_api_happy_path_playbook,
    calibration_api_flaky_path_playbook, reset_validation_db_playbook,
};

pub async fn create_arena() -> OpenArena {
    let dependencies = setup_dependencies();
    let mssql_id = MSSQL_ID.get().expect("mssql id initialized").clone();
    let calibration_id = CALIBRATION_ID
        .get()
        .expect("calibration id initialized")
        .clone();
    let exec_component = setup_exec_component();
    let components: Vec<Component> = vec![exec_component];

    let matches: Vec<Box<dyn MatchTrait>> = vec![Box::new(
        Match::new("example-api-happy-path", dependencies, components)
            .register_playbook(
                Box::new(calibration_api_happy_path_playbook(calibration_id.clone())),
                true,
            )
            .register_playbook(
                Box::new(calibration_api_error_path_playbook(calibration_id.clone())),
                false,
            )
            .register_playbook(
                Box::new(calibration_api_flaky_path_playbook(calibration_id)),
                false,
            )
            .register_playbook(reset_validation_db_playbook(mssql_id).into_box(), false),
    )];
    let closed_arena = ClosedArena::new("example-api-arena".to_string(), matches);

    closed_arena.open().await
}

static SHARED_ARENA: OnceCell<OpenArena> = OnceCell::const_new();

pub static SCENARIO_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

static AXUM_COMPONENT_RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn axum_component_runtime() -> &'static Runtime {
    AXUM_COMPONENT_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("component test tokio runtime")
    })
}

#[ctor::ctor]
fn install_tracing_for_axum_component_tests() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
}

pub async fn shared_arena() -> &'static OpenArena {
    SHARED_ARENA
        .get_or_init(|| async { create_arena().await })
        .await
}

#[ctor::dtor]
unsafe fn teardown() {
    if let Some(arena) = SHARED_ARENA.get() {
        let _guard = axum_component_runtime().enter();
        std::ptr::drop_in_place(arena as *const OpenArena as *mut OpenArena);
    }
}
