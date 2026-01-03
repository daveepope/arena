use arena::dependency::RunnableDependency;
use async_trait::async_trait;
use testcontainers_modules::{postgres, testcontainers, testcontainers::runners::AsyncRunner};

#[async_trait]
pub trait PostgresDependencyWrapper: Send + Sync {
    async fn start(&mut self, name: &str);
    async fn stop(&mut self, name: &str);

    fn connection_string(&self) -> Option<&str>;
}

pub struct PostgresDependency {
    pub name: String,
    pg: Box<dyn PostgresDependencyWrapper>,
    dependencies: Vec<Box<dyn RunnableDependency>>,
    running: bool,
}

impl PostgresDependency {
    pub fn new(name: String, postgres_wrapper: Box<dyn PostgresDependencyWrapper>) -> Self {
        Self { name, pg: postgres_wrapper, dependencies: vec![], running: false }
    }

    pub fn connection_string(&self) -> Option<&str> {
        self.pg.connection_string()
    }
}

#[async_trait]
impl RunnableDependency for PostgresDependency {
    async fn start(&mut self) {
        if self.running {
            return;
        }

        log::info!("[Postgres-{}] starting.", self.name);

        for dep in self.dependencies.iter_mut() {
            dep.start().await;
        }

        self.pg.start(&self.name).await;

        self.running = true;
        log::info!("[Postgres-{}] started.", self.name);
    }

    async fn stop(&mut self) {
        if !self.running {
            return;
        }

        log::info!("[Postgres-{}] stopping.", self.name);

        self.pg.stop(&self.name).await;

        for dep in self.dependencies.iter_mut().rev() {
            dep.stop().await;
        }

        self.running = false;
        log::info!("[Postgres-{}] stopped.", self.name);
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.push(dep);
    }
}

pub struct InternalPostgresTestContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<postgres::Postgres>>,
    conn: Option<String>,
}

impl InternalPostgresTestContainerImpl {
    pub fn new() -> Self {
        Self { container: None, conn: None }
    }
}

#[async_trait]
impl PostgresDependencyWrapper for InternalPostgresTestContainerImpl {
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
            .get_host_port_ipv4(5432).await.expect("Failed to get port").to_string();

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