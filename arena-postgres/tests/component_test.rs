use arena::dependency::RunnableDependency;
use arena_postgres::PostgresDependency;
use futures::FutureExt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn init_test_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
}

fn ephemeral_tcp_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral tcp port")
        .local_addr()
        .expect("local_addr")
        .port()
}

fn with_client<F, T>(conn_str: &str, f: F) -> Result<T, String>
where
    F: FnOnce(&mut postgres::Client) -> Result<T, String>,
{
    let mut client = postgres::Client::connect(conn_str, postgres::NoTls)
        .map_err(|e| format!("connect failed: {e}"))?;
    f(&mut client)
}

struct TestContext {
    pg: PostgresDependency,
    conn_str: String,
    table: String,
}

impl TestContext {
    async fn new() -> Result<Self, String> {
        tracing::info!(
            suite = "crate_component",
            crate_under_test = "arena_postgres",
            phase = "dependency_start_begin",
            "starting dependency",
        );
        let mut pg = PostgresDependency::builder("")
            .with_port(ephemeral_tcp_port())
            .build();
        pg.start().await;

        let conn_str = pg
            .connection_string()
            .ok_or_else(|| "postgres connection string missing after start()".to_string())?
            .to_string();

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let table = format!("arena_component_test_{ts}");

        Ok(Self {
            pg,
            conn_str,
            table,
        })
    }

    async fn create_table(&self) -> Result<(), String> {
        let conn_str = self.conn_str.clone();
        let table = self.table.clone();
        tokio::task::spawn_blocking(move || {
            with_client(&conn_str, |c| {
                c.batch_execute(&format!(
                    r#"
                    create table if not exists "{table}"(
                        id serial primary key,
                        v int not null
                    )
                    "#
                ))
                .map_err(|e| format!("create table failed: {e}"))
            })
        })
        .await
        .map_err(|e| format!("create table task join failed: {e}"))?
    }

    async fn insert_and_count(&self) -> Result<i64, String> {
        let conn_str = self.conn_str.clone();
        let table = self.table.clone();
        tokio::task::spawn_blocking(move || {
            with_client(&conn_str, |c| {
                c.execute(
                    &format!(r#"insert into "{table}"(v) values ($1)"#),
                    &[&123i32],
                )
                .map_err(|e| format!("insert failed: {e}"))?;

                let row = c
                    .query_one(&format!(r#"select count(*) from "{table}""#), &[])
                    .map_err(|e| format!("count query failed: {e}"))?;
                Ok(row.get::<_, i64>(0))
            })
        })
        .await
        .map_err(|e| format!("insert/count task join failed: {e}"))?
    }

    async fn drop_table_best_effort(&self) {
        let conn_str = self.conn_str.clone();
        let table = self.table.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let _ = with_client(&conn_str, |c| {
                c.batch_execute(&format!(r#"drop table if exists "{table}""#))
                    .map_err(|e| format!("drop table failed: {e}"))
            });
        })
        .await;
    }

    async fn stop(mut self) {
        self.pg.stop().await;
    }
}

#[tokio::test]
async fn postgres_dependency_lifecycle_component_test() {
    init_test_logging();

    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_postgres",
        phase = "db_roundtrip_begin",
        table = %ctx.table,
        "begin db roundtrip",
    );
    let outcome = std::panic::AssertUnwindSafe(async {
        ctx.create_table().await?;
        let count = ctx.insert_and_count().await?;
        if count < 1 {
            return Err(format!("expected count >= 1, got {count}"));
        }
        Ok::<(), String>(())
    })
    .catch_unwind()
    .await;

    ctx.drop_table_best_effort().await;

    tokio::time::timeout(Duration::from_secs(10), ctx.stop())
        .await
        .unwrap_or_else(|_| panic!("postgres stop timed out"));

    match outcome {
        Ok(Ok(())) => tracing::info!(
            suite = "crate_component",
            crate_under_test = "arena_postgres",
            phase = "db_roundtrip_ok",
            "scenario passed",
        ),
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}

async fn playbook_scenario(pg: &PostgresDependency) -> Result<(), String> {
    let conn_str = pg
        .connection_string()
        .ok_or_else(|| "connection string missing".to_string())?
        .to_string();

    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_postgres",
        scenario = "playbook",
        phase = "begin",
        "begin playbook scenario",
    );

    {
        let conn_str = conn_str.clone();
        tokio::task::spawn_blocking(move || {
            with_client(&conn_str, |c| {
                c.batch_execute("insert into widgets (name) values ('alpha'), ('beta'), ('gamma');")
                    .map_err(|e| format!("seed insert failed: {e}"))
            })
        })
        .await
        .map_err(|e| format!("seed task join failed: {e}"))??;
    }

    let playbook = pg.playbook().run().await;

    let count = playbook.verify("SELECT COUNT(*) FROM widgets;").await;
    if count != 0 {
        return Err(format!(
            "expected playbook to clear widgets, got count={count}"
        ));
    }

    {
        let conn_str = conn_str.clone();
        tokio::task::spawn_blocking(move || {
            with_client(&conn_str, |c| {
                c.batch_execute("insert into widgets (name) values ('delta'), ('epsilon');")
                    .map_err(|e| format!("second seed failed: {e}"))
            })
        })
        .await
        .map_err(|e| format!("second seed task join failed: {e}"))??;
    }

    let playbook = pg.playbook().run().await;
    let count = playbook.verify("SELECT COUNT(*) FROM widgets;").await;
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
        crate_under_test = "arena_postgres",
        scenario = "playbook",
        phase = "ok",
        "playbook scenario finished",
    );
    Ok(())
}

#[tokio::test]
async fn postgres_dependency_playbook_component_test() {
    init_test_logging();

    let mut pg = PostgresDependency::builder("postgres-playbook-component")
        .with_port(ephemeral_tcp_port())
        .with_startup_sql_scripts(vec![r#"
            create table if not exists widgets(
                id serial primary key,
                name text not null
            )
            "#
        .to_string()])
        .build();

    pg.start().await;

    let outcome = std::panic::AssertUnwindSafe(playbook_scenario(&pg))
        .catch_unwind()
        .await;

    tokio::time::timeout(Duration::from_secs(10), pg.stop())
        .await
        .unwrap_or_else(|_| panic!("postgres stop timed out"));

    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}
