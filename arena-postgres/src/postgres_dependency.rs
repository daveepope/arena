use arena::dependency::RunnableDependency;
use async_trait::async_trait;
use crate::builder::PostgresDependencyBuilder;

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

    pub fn builder(name: impl Into<String>) -> PostgresDependencyBuilder {
        PostgresDependencyBuilder::new(name)
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