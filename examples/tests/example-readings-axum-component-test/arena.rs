use arena::{ClosedArena, Component, Dependency, Match, MatchTrait, OpenArena};
use arena_examples::http_healthcheck::HttpReadinessCheck;
use arena_executable_component::executable_component::ExecutableComponent;
use arena_http::{ok_json, HttpDependency, ManagedHttpPlaybook};
use arena_kafka::{KafkaDependency, KafkaFlavor};
use arena_mssql::{ManagedMssqlPlaybook, MssqlDependency};
use arena_oauth::OauthDependency;
use arena_postgres::PostgresDependency;
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::OnceLock;
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;

pub static KAFKA_ID: OnceLock<String> = OnceLock::new();
pub static CALIBRATION_ID: OnceLock<String> = OnceLock::new();
pub static MSSQL_ID: OnceLock<String> = OnceLock::new();
pub static OAUTH_ID: OnceLock<String> = OnceLock::new();

pub const OAUTH_PORT: u16 = 9443;
pub const OAUTH_ISSUER: &str = "https://127.0.0.1:9443";
pub const OAUTH_TLS_CERT_PEM: &str = include_str!("../../resources/oauth_tls_cert.pem");
const OAUTH_TLS_KEY_PEM: &str = include_str!("../../resources/oauth_tls_key.pem");

pub const POSTGRES_PORT: u16 = 5555;
pub const POSTGRES_DB_NAME: &str = "readings_db";
pub const POSTGRES_DB_USER: &str = "readings_user";
pub const POSTGRES_DB_PASS: &str = "readings_password";
pub const KAFKA_PORT: u16 = 9093;
pub const CALIBRATION_HTTP_PORT: u16 = 8888;
pub const EXEC_WEB_APP_PORT: u16 = 3000;
pub const MSSQL_PORT: u16 = 1435;
pub const MSSQL_DB_NAME: &str = "validationDb";
pub const MSSQL_DB_USER: &str = "sa";
pub const MSSQL_DB_PASS: &str = "yourStrong(!)Password";

pub const NETWORK_NAME: &str = "arena-component-test-network";

pub fn init_logging() {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,arena=debug"),
    )
    .is_test(true)
    .try_init();
}

pub fn setup_dependencies() -> Vec<Dependency> {
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

pub fn resolve_web_app_binary() -> String {
    if let Ok(runfiles) = std::env::var("RUNFILES_DIR") {
        return format!("{runfiles}/_main/examples/example-readings-axum-web-app");
    }
    "target/release/example-readings-axum-web-app".to_string()
}

pub fn setup_exec_component() -> Component {
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
        .with_runtime_arg(
            "calibration_url",
            format!("http://127.0.0.1:{}", CALIBRATION_HTTP_PORT),
        )
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

pub fn calibration_default_playbook() -> ManagedHttpPlaybook {
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

pub async fn create_arena() -> OpenArena {
    let dependencies = setup_dependencies();
    let mssql_id = MSSQL_ID.get().expect("mssql id initialized").clone();
    let exec_component = setup_exec_component();
    let components: Vec<Component> = vec![exec_component];

    let matches: Vec<Box<dyn MatchTrait>> = vec![Box::new(
        Match::new("reading lifecycle", dependencies, components)
            .register_playbook(Box::new(calibration_default_playbook()), true)
            .register_playbook(
                ManagedMssqlPlaybook::new("axum-readings-mssql-session", mssql_id).into_box(),
                true,
            ),
    )];
    let closed_arena = ClosedArena::new("Test Arena".to_string(), matches);

    closed_arena.open().await
}

static SHARED_ARENA: OnceCell<OpenArena> = OnceCell::const_new();

pub static SCENARIO_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

static READINGS_AXUM_COMPONENT_RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn readings_axum_component_runtime() -> &'static Runtime {
    READINGS_AXUM_COMPONENT_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("component test tokio runtime")
    })
}

pub async fn shared_arena() -> &'static OpenArena {
    init_logging();
    SHARED_ARENA
        .get_or_init(|| async { create_arena().await })
        .await
}

#[ctor::dtor]
unsafe fn teardown() {
    if let Some(arena) = SHARED_ARENA.get() {
        let _guard = readings_axum_component_runtime().enter();
        std::ptr::drop_in_place(arena as *const OpenArena as *mut OpenArena);
    }
}
