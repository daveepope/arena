use arena::dependency::RunnableDependency;
use async_trait::async_trait;
use testcontainers_modules::{postgres, testcontainers, testcontainers::runners::AsyncRunner};

#[async_trait]
pub trait PostgresImpl: Send + Sync {
    async fn start(&mut self, name: &str);
    async fn stop(&mut self, name: &str);

    fn connection_string(&self) -> Option<&str>;
}

pub struct DockerPostgresImpl {
    // Keeping this handle alive keeps the container running
    container: Option<testcontainers::core::Container<postgres::Postgres>>,
    conn: Option<String>,
}

impl DockerPostgresImpl {
    pub fn new() -> Self {
        Self { container: None, conn: None }
    }
}

#[async_trait]
impl PostgresImpl for DockerPostgresImpl {
    async fn start(&mut self, name: &str) {
        if self.container.is_some() {
            return;
        }

        let container = postgres::Postgres::default()
            .start()
            .await
            .expect("start postgres container");

        // NOTE: depending on versions, these may or may not be async. If the compiler
        // says “not a future”, just remove `.await`.
        let host = container.get_host().to_string();
        let port = container
            .get_host_port_ipv4(5432)
            .expect("get mapped postgres port");

        self.conn = Some(format!("postgres://postgres:postgres@{host}:{port}/postgres"));
        self.container = Some(container);

        println!("[PostgresImpl-{}] started container.", name);
    }

    async fn stop(&mut self, name: &str) {
        self.container.take();
        self.conn = None;
        println!("[PostgresImpl-{}] stopped container.", name);
    }

    fn connection_string(&self) -> Option<&str> {
        self.conn.as_deref()
    }
}

pub struct PostgresDependency {
    pub name: String,
    pg: Box<dyn PostgresImpl>,
    dependencies: Vec<Box<dyn RunnableDependency>>,
    running: bool,
}

impl PostgresDependency {
    pub fn new(name: String, pg: Box<dyn PostgresImpl>) -> Self {
        Self { name, pg, dependencies: vec![], running: false }
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

        println!("[Postgres-{}] starting.", self.name);

        for dep in self.dependencies.iter_mut() {
            dep.start().await;
        }

        self.pg.start(&self.name).await;

        self.running = true;
        println!("[Postgres-{}] started.", self.name);
    }

    async fn stop(&mut self) {
        if !self.running {
            return;
        }

        println!("[Postgres-{}] stopping.", self.name);

        self.pg.stop(&self.name).await;

        for dep in self.dependencies.iter_mut().rev() {
            dep.stop().await;
        }

        self.running = false;
        println!("[Postgres-{}] stopped.", self.name);
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.push(dep);
    }
}