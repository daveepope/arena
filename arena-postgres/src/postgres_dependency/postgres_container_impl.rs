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
        image_tag: &str,
        container_name: &str,
    );
    async fn stop(&mut self);

    fn connection_string(&self) -> Option<&str>;
}

pub(crate) struct PostgresContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<postgres::Postgres>>,
    connection_string: Option<String>,
}

impl PostgresContainerImpl {
    pub(crate) fn new() -> Self {
        Self { container: None, connection_string: None }
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
        image_tag: &str,
        container_name: &str,
    ) {
        if self.container.is_some() {
            return;
        }

        const DEFAULT_CONTAINER_PORT: u16 = 5432;

        let healthcheck = Healthcheck::cmd_shell(format!(
            "pg_isready -h 127.0.0.1 -p {DEFAULT_CONTAINER_PORT} -U {database_username} -d {database_name}"
        ))
        .with_interval(Duration::from_millis(250))
        .with_timeout(Duration::from_secs(1))
        .with_retries(40u32);

        let container = postgres::Postgres::default()
            .with_db_name(database_name)
            .with_user(database_username)
            .with_password(database_password)
            .with_mapped_port(port, ContainerPort::from(DEFAULT_CONTAINER_PORT))
            .with_health_check(healthcheck)
            .with_tag(image_tag)
            .with_container_name(container_name)
            .start()
            .await
            .expect("start postgres container");

        let host = container
            .get_host()
            .await
            .expect("Failed to get host")
            .to_string();

        let port = container
            .get_host_port_ipv4(DEFAULT_CONTAINER_PORT)
            .await
            .expect("Failed to get port")
            .to_string();
        self.connection_string = Some(format!(
            "postgres://{database_username}:{database_password}@{host}:{port}/{database_name}"
        ));
        self.container = Some(container);

        log::info!("[PostgresImpl] started container.");
    }

    async fn stop(&mut self) {
        self.container.take();
        self.connection_string = None;
        log::info!("[PostgresImpl] stopped container.");
    }

    fn connection_string(&self) -> Option<&str> {
        self.connection_string.as_deref()
    }
}