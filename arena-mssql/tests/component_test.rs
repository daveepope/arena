use arena::dependency::RunnableDependency;
use arena_mssql::MssqlDependency;
use std::time::{SystemTime, UNIX_EPOCH};
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

const EPHEMERAL_PORT_RANGE: std::ops::RangeInclusive<u16> = 21250..=21299;

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

async fn open_client(conn_str: &str) -> Result<Client<Compat<TcpStream>>, String> {
    let mut config = Config::from_ado_string(conn_str).map_err(|e| format!("parse ado: {e}"))?;
    config.trust_cert();
    let tcp = TcpStream::connect(config.get_addr())
        .await
        .map_err(|e| format!("tcp: {e}"))?;
    tcp.set_nodelay(true).map_err(|e| format!("nodelay: {e}"))?;
    Client::connect(config, tcp.compat_write())
        .await
        .map_err(|e| format!("connect: {e}"))
}

async fn lifecycle_scenario(mssql: &MssqlDependency) -> Result<(), String> {
    let conn_str = mssql
        .connection_string()
        .ok_or_else(|| "connection string missing".to_string())?
        .to_string();

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let table = format!("arena_component_test_{ts}");

    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_mssql",
        scenario = "lifecycle",
        table = %table,
        phase = "begin",
        "begin lifecycle scenario",
    );

    let mut client = open_client(&conn_str).await?;

    let create_sql = format!(
        r#"
        IF OBJECT_ID(N'dbo.[{table}]', N'U') IS NULL
        CREATE TABLE dbo.[{table}] (
            id INT IDENTITY(1,1) PRIMARY KEY,
            v  INT NOT NULL
        );
        "#
    );
    client
        .simple_query(create_sql.as_str())
        .await
        .map_err(|e| format!("create table failed: {e}"))?;

    let insert_sql = format!(r#"INSERT INTO dbo.[{table}] (v) VALUES (123);"#);
    client
        .simple_query(insert_sql.as_str())
        .await
        .map_err(|e| format!("insert failed: {e}"))?;

    let count_sql = format!(r#"SELECT CAST(COUNT_BIG(*) AS BIGINT) FROM dbo.[{table}];"#);
    let stream = client
        .simple_query(count_sql.as_str())
        .await
        .map_err(|e| format!("count query failed: {e}"))?;
    let row = stream
        .into_row()
        .await
        .map_err(|e| format!("count read failed: {e}"))?
        .ok_or_else(|| "count returned no rows".to_string())?;

    let count: i64 = row
        .get::<i64, _>(0)
        .ok_or_else(|| "count was null".to_string())?;
    if count < 1 {
        return Err(format!("expected count >= 1, got {count}"));
    }

    let drop_sql =
        format!(r#"IF OBJECT_ID(N'dbo.[{table}]', N'U') IS NOT NULL DROP TABLE dbo.[{table}];"#);
    let _ = client.simple_query(drop_sql.as_str()).await;

    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_mssql",
        scenario = "lifecycle",
        phase = "ok",
        "lifecycle scenario finished",
    );
    Ok(())
}

async fn playbook_scenario(mssql: &MssqlDependency) -> Result<(), String> {
    let conn_str = mssql
        .connection_string()
        .ok_or_else(|| "connection string missing".to_string())?
        .to_string();

    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_mssql",
        scenario = "playbook",
        phase = "begin",
        "begin playbook scenario",
    );

    {
        let mut client = open_client(&conn_str).await?;
        client
            .simple_query(
                "INSERT INTO dbo.widgets (name) VALUES (N'alpha'), (N'beta'), (N'gamma');",
            )
            .await
            .map_err(|e| format!("seed insert failed: {e}"))?;
    }

    let playbook = mssql.playbook().run().await;

    let count = playbook.verify("SELECT COUNT(*) FROM dbo.widgets;").await;
    if count != 0 {
        return Err(format!(
            "expected playbook to clear widgets, got count={count}"
        ));
    }

    {
        let mut client = open_client(&conn_str).await?;
        client
            .simple_query("INSERT INTO dbo.widgets (name) VALUES (N'delta'), (N'epsilon');")
            .await
            .map_err(|e| format!("second seed failed: {e}"))?;
    }

    let playbook = mssql.playbook().run().await;
    let count = playbook.verify("SELECT COUNT(*) FROM dbo.widgets;").await;
    if count != 0 {
        return Err(format!(
            "expected playbook to clear widgets again, got count={count}"
        ));
    }

    let literal = playbook.verify("SELECT 1 + 1;").await;
    if literal != 2 {
        return Err(format!("expected verify('SELECT 1+1') == 2, got {literal}"));
    }

    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_mssql",
        scenario = "playbook",
        phase = "ok",
        "playbook scenario finished",
    );
    Ok(())
}

#[tokio::test]
async fn mssql_dependency_component_test() {
    init_test_logging();

    let mut mssql = MssqlDependency::builder("mssql-component")
        .with_port(ephemeral_tcp_port())
        .with_startup_sql_scripts(vec![r#"
            IF OBJECT_ID(N'dbo.widgets', N'U') IS NULL
            CREATE TABLE dbo.widgets (
                id INT IDENTITY(1,1) PRIMARY KEY,
                name NVARCHAR(64) NOT NULL
            );
            "#
        .to_string()])
        .build();

    mssql.start().await;

    if let Err(e) = lifecycle_scenario(&mssql).await {
        panic!("lifecycle scenario: {e}");
    }

    if let Err(e) = playbook_scenario(&mssql).await {
        panic!("playbook scenario: {e}");
    }
}
