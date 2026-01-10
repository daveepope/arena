use async_trait::async_trait;
use testcontainers_modules::{postgres, testcontainers, testcontainers::runners::AsyncRunner};

use crate::postgres_dependency::PostgresDependencyWrapper;

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
impl PostgresDependencyWrapper for PostgresContainerImpl {
    async fn start(&mut self, name: &str) {
        if self.container.is_some() {
            return;
        }

        let container = postgres::Postgres::default()
            .start()
            .await
            .expect("start postgres container");

        let host = container.get_host().await.expect("Failed to get host").to_string();
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("Failed to get port")
            .to_string();

        self.conn = Some(format!("postgres://postgres:postgres@{host}:{port}/postgres"));
        self.container = Some(container);

        log::info!("[PostgresImpl-{}] started container.", name);
    }

    async fn stop(&mut self, name: &str) {
        self.container.take();
        self.conn = None;
        log::info!("[PostgresImpl-{}] stopped container.", name);
    }

    fn connection_string(&self) -> Option<&str> {
        self.conn.as_deref()
    }
}