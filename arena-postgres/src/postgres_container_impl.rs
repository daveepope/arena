use async_trait::async_trait;
use std::time::Duration;
use testcontainers_modules::{postgres, testcontainers, testcontainers::runners::AsyncRunner};
use testcontainers_modules::testcontainers::core::{ContainerPort, Healthcheck};
use testcontainers_modules::testcontainers::ImageExt;

#[async_trait]
pub trait PostgresImpl: Send + Sync {
    async fn start(
        &mut self,
        port: u16,
        database_name: &str,
        database_username: &str,
        database_password: &str,
    );
    async fn stop(&mut self);

    fn connection_string(&self) -> Option<&str>;
}

pub(crate) struct PostgresContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<postgres::Postgres>>,
    conn: Option<String>,
}

impl PostgresContainerImpl {
    pub(crate) fn new() -> Self {
        Self { container: None, conn: None }
    }
}

#[async_trait]
impl PostgresImpl for PostgresContainerImpl {
    async fn start(
        &mut self,
        port: u16,
        database_name: &str,
        database_username: &str,
        database_password: &str,
    ) {
        if self.container.is_some() {
            return;
        }

        const CONTAINER_PORT: u16 = 5432;

        let healthcheck = Healthcheck::cmd_shell(format!(
            "pg_isready -h 127.0.0.1 -p {CONTAINER_PORT} -U {database_username} -d {database_name}"
        ))
        .with_interval(Duration::from_millis(250))
        .with_timeout(Duration::from_secs(1))
        // 10s / 250ms = 40 attempts (ish)
        .with_retries(40u32);

        let container = postgres::Postgres::default()
            .with_db_name(database_name)
            .with_user(database_username)
            .with_password(database_password)
            .with_mapped_port(port, ContainerPort::from(5432))
            .with_health_check(healthcheck)
            .start()
            .await
            .expect("start postgres container");

        let host = container
            .get_host()
            .await
            .expect("Failed to get host")
            .to_string();

        let port = container
            .get_host_port_ipv4(CONTAINER_PORT)
            .await
            .expect("Failed to get port")
            .to_string();

        self.conn = Some(format!(
            "postgres://{database_username}:{database_password}@{host}:{port}/{database_name}"
        ));
        self.container = Some(container);

        log::info!("[PostgresImpl] started container.");
    }

    async fn stop(&mut self) {
        self.container.take();
        self.conn = None;
        log::info!("[PostgresImpl] stopped container.");
    }

    fn connection_string(&self) -> Option<&str> {
        self.conn.as_deref()
    }
}