use arena::dependency::RunnableDependency;
use async_trait::async_trait;
use crate::builder::PostgresDependencyBuilder;
use crate::postgres_container_impl::PostgresImpl;

pub struct PostgresDependency {
    pub name: String,
    postgres_impl: Box<dyn PostgresImpl>,
    port: u16,
    startup_sql_scripts: Option<Vec<String>>,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    running: bool,
}

impl PostgresDependency {
    pub fn new(name: String, postgres_impl: Box<dyn PostgresImpl>, port : u16, startup_sql_scripts: Option<Vec<String>>, dependencies: Option<Vec<Box<dyn RunnableDependency>>>) -> Self {
        Self { name, postgres_impl, port, startup_sql_scripts, dependencies, running: false }
    }

    pub fn connection_string(&self) -> Option<&str> {
        self.postgres_impl.connection_string()
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

        log::info!("[PostgresDependency-{}] starting.", self.name);

        for dep in self.dependencies.iter_mut().flatten() {
            dep.start().await;
        }

        let scripts = self.startup_sql_scripts.take();
        self.postgres_impl.start(&self.name, self.port, scripts).await;

        self.running = true;
        log::info!("[PostgresDependency-{}] started.", self.name);
    }

    async fn stop(&mut self) {
        if !self.running {
            return;
        }

        log::info!("[PostgresDependency-{}] stopping.", self.name);

        self.postgres_impl.stop(&self.name).await;

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.stop().await;
        }

        self.running = false;
        log::info!("[PostgresDependency-{}] stopped.", self.name);
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.get_or_insert_with(Vec::new).push(dep);
    }
}