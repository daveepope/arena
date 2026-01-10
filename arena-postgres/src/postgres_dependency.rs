use arena::dependency::RunnableDependency;
use async_trait::async_trait;
use crate::builder::PostgresDependencyBuilder;
use crate::postgres_container_impl::PostgresImpl;
use tokio::time::{Duration, Instant};

pub struct PostgresDependency {
    pub identifier: String,
    postgres_impl: Box<dyn PostgresImpl>,
    port: u16,
    database_name: String,
    database_username: String,
    database_password: String,
    startup_sql_scripts: Option<Vec<String>>,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    running: bool,
}

impl PostgresDependency {
    pub fn new(
        identifier: String,
        postgres_impl: Box<dyn PostgresImpl>,
        port: u16,
        database_name: String,
        database_username: String,
        database_password: String,
        startup_sql_scripts: Option<Vec<String>>,
        dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    ) -> Self {
        Self {
            identifier,
            postgres_impl,
            port,
            database_name,
            database_username,
            database_password,
            startup_sql_scripts,
            dependencies,
            running: false,
        }
    }

    pub fn connection_string(&self) -> Option<&str> {
        self.postgres_impl.connection_string()
    }

    pub fn builder(identifier: impl Into<String>) -> PostgresDependencyBuilder {
        PostgresDependencyBuilder::new(identifier)
    }

    async fn run_startup_sql_scripts(&self, scripts: Vec<String>) {
        let conn_str = self
            .connection_string()
            .expect("connection string should be available after postgres starts");

        let (client, connection) = tokio_postgres::connect(conn_str, tokio_postgres::NoTls)
            .await
            .expect("connect to postgres to run startup scripts");

        tokio::spawn(async move {
            if let Err(err) = connection.await {
                log::warn!(
                    "[PostgresDependency] postgres connection error while running startup scripts: {err}"
                );
            }
        });

        log::info!(
            "[PostgresDependency-{}] running {} startup sql script(s).",
            self.identifier,
            scripts.len()
        );

        for (idx, sql) in scripts.iter().enumerate() {
            log::info!(
                "[PostgresDependency-{}] running startup sql script {}/{}.",
                self.identifier,
                idx + 1,
                scripts.len()
            );

            client
                .batch_execute(sql)
                .await
                .unwrap_or_else(|err| {
                    panic!(
                        "[PostgresDependency-{}] startup sql script {}/{} failed: {err}",
                        self.identifier,
                        idx + 1,
                        scripts.len()
                    )
                });
        }

        log::info!(
            "[PostgresDependency-{}] startup sql scripts complete.",
            self.identifier
        );
    }

    async fn is_ready_once(&self) -> bool {
        let conn_str = match self.connection_string() {
            Some(s) => s,
            None => return false,
        };

        let (client, connection) = match tokio_postgres::connect(conn_str, tokio_postgres::NoTls).await
        {
            Ok(v) => v,
            Err(_) => return false,
        };

        // Drive the connection in the background for the duration of this check.
        tokio::spawn(async move {
            if let Err(err) = connection.await {
                log::debug!("[PostgresDependency] connection error while checking readiness: {err}");
            }
        });

        client.simple_query("SELECT 1").await.is_ok()
    }
}

#[async_trait]
impl RunnableDependency for PostgresDependency {
    async fn start(&mut self) {
        if self.running {
            return;
        }

        log::info!("[PostgresDependency-{}] starting.", self.identifier);

        for dep in self.dependencies.iter_mut().flatten() {
            dep.start().await;
        }

        let scripts = self.startup_sql_scripts.take();
        let database_name = self.database_name.clone();
        let database_username = self.database_username.clone();
        let database_password = self.database_password.clone();

        self.postgres_impl
            .start(
                self.port,
                &database_name,
                &database_username,
                &database_password,
            )
            .await;

        let timeout = Duration::from_secs(10);
        let poll_every = Duration::from_millis(250);
        let start = Instant::now();

        while !self.is_ready_once().await {
            if start.elapsed() >= timeout {
                panic!(
                    "[PostgresDependency-{}] postgres did not become ready within {:?}. conn_str={:?}",
                    self.identifier,
                    timeout,
                    self.connection_string(),
                );
            }

            tokio::time::sleep(poll_every).await;
        }

        if let Some(scripts) = scripts {
            self.run_startup_sql_scripts(scripts).await;
        }

        self.running = true;
        log::info!("[PostgresDependency-{}] started and ready.", self.identifier);
    }

    async fn stop(&mut self) {
        if !self.running {
            return;
        }

        log::info!("[PostgresDependency-{}] stopping.", self.identifier);

        self.postgres_impl.stop().await;

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.stop().await;
        }

        self.running = false;
        log::info!("[PostgresDependency-{}] stopped.", self.identifier);
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.get_or_insert_with(Vec::new).push(dep);
    }
}