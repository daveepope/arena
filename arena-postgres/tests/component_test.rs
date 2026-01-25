use arena::dependency::RunnableDependency;
use arena_postgres::PostgresDependency;
use futures::FutureExt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn init_test_logging() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .is_test(true)
        .try_init();
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
        log::info!("[component-test] starting PostgresDependency");
        let mut pg = PostgresDependency::builder("arena-postgres component test").build();
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

        Ok(Self { pg, conn_str, table })
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

    log::info!("[component-test] db roundtrip begin (table={})", ctx.table);
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
        Ok(Ok(())) => log::info!("[component-test] ok"),
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}