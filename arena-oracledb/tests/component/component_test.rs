use arena::dependency::RunnableDependency;
use arena_oracledb::OracleDependency;
use std::time::{SystemTime, UNIX_EPOCH};

const EPHEMERAL_PORT_RANGE: std::ops::RangeInclusive<u16> = 21400..=21449;

fn ephemeral_tcp_port() -> u16 {
    arena_host::find_available_port::find_available_port(
        EPHEMERAL_PORT_RANGE,
        arena_host::find_available_port::PortSearchStrategy::Random,
    )
    .unwrap_or_else(|| {
        panic!(
            "no available port found in range {}..={}",
            EPHEMERAL_PORT_RANGE.start(), EPHEMERAL_PORT_RANGE.end()
        )
    })
}

fn init_test_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
}

async fn lifecycle_scenario(oracle: &OracleDependency) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let table = format!("arena_component_test_{ts}");

    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_oracledb",
        scenario = "lifecycle",
        table = %table,
        phase = "begin",
        "begin lifecycle scenario",
    );

    oracle
        .execute(&format!(
            "CREATE TABLE {table} (id NUMBER GENERATED ALWAYS AS IDENTITY PRIMARY KEY, v NUMBER NOT NULL);"
        ))
        .await;

    oracle.execute(&format!("INSERT INTO {table} (v) VALUES (123);")).await;

    let count = oracle
        .query_scalar(&format!("SELECT COUNT(*) FROM {table};"))
        .await;
    assert!(count >= 1, "expected count >= 1, got {count}");

    oracle.execute(&format!("DROP TABLE {table};")).await;

    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_oracledb",
        scenario = "lifecycle",
        phase = "ok",
        "lifecycle scenario finished",
    );
}

async fn playbook_scenario(oracle: &OracleDependency) {
    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_oracledb",
        scenario = "playbook",
        phase = "begin",
        "begin playbook scenario",
    );

    oracle
        .execute("INSERT INTO widgets (name) VALUES ('alpha');")
        .await;
    oracle
        .execute("INSERT INTO widgets (name) VALUES ('beta');")
        .await;
    oracle
        .execute("INSERT INTO widgets (name) VALUES ('gamma');")
        .await;

    let playbook = oracle.playbook().run().await;

    let count = playbook.verify("SELECT COUNT(*) FROM widgets;").await;
    assert_eq!(count, 0, "expected playbook to clear widgets, got count={count}");

    oracle
        .execute("INSERT INTO widgets (name) VALUES ('delta');")
        .await;
    oracle
        .execute("INSERT INTO widgets (name) VALUES ('epsilon');")
        .await;

    let playbook = oracle.playbook().run().await;
    let count = playbook.verify("SELECT COUNT(*) FROM widgets;").await;
    assert_eq!(
        count, 0,
        "expected playbook to clear widgets again, got count={count}"
    );

    let literal = playbook.verify("SELECT 1 + 1 FROM dual;").await;
    assert_eq!(literal, 2, "expected verify('SELECT 1+1') == 2, got {literal}");

    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_oracledb",
        scenario = "playbook",
        phase = "ok",
        "playbook scenario finished",
    );
}

#[tokio::test]
async fn oracle_dependency_component_test() {
    init_test_logging();

    let mut oracle = OracleDependency::builder("oracle-component")
        .with_port(ephemeral_tcp_port())
        .with_startup_sql_scripts(vec![
            "CREATE TABLE widgets (\n\
             id NUMBER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,\n\
             name VARCHAR2(64) NOT NULL\n\
             );"
                .to_string(),
        ])
        .build();

    oracle.start().await;

    lifecycle_scenario(&oracle).await;
    playbook_scenario(&oracle).await;
}
