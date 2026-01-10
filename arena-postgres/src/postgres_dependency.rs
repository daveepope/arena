use arena::dependency::RunnableDependency;
use async_trait::async_trait;
use crate::builder::PostgresDependencyBuilder;
use crate::postgres_container_impl::PostgresImpl;
use futures::channel::oneshot;
use std::time::{Duration, Instant};

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

    fn run_startup_sql_scripts(identifier: &str, conn_str: &str, scripts: &[String]) {
        let mut client = postgres::Client::connect(conn_str, postgres::NoTls)
            .expect("connect to postgres to run startup scripts");

        log::info!(
            "[PostgresDependency-{}] running {} startup sql script(s).",
            identifier,
            scripts.len()
        );

        for (idx, sql) in scripts.iter().enumerate() {
            log::info!(
                "[PostgresDependency-{}] running startup sql script {}/{}.",
                identifier,
                idx + 1,
                scripts.len()
            );

            client
                .batch_execute(sql)
                .unwrap_or_else(|err| {
                    panic!(
                        "[PostgresDependency-{}] startup sql script {}/{} failed: {err}",
                        identifier,
                        idx + 1,
                        scripts.len()
                    )
                });
        }

        log::info!(
            "[PostgresDependency-{}] startup sql scripts complete.",
            identifier
        );
    }

    fn is_ready_once(conn_str: &str) -> bool {
        let mut client = match postgres::Client::connect(conn_str, postgres::NoTls) {
            Ok(v) => v,
            Err(_) => return false,
        };

        client.simple_query("SELECT 1").is_ok()
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

        let identifier = self.identifier.clone();
        let conn_str = self
            .connection_string()
            .expect("connection string should be available after postgres starts")
            .to_string();

        let (tx, rx) = oneshot::channel::<Result<(), String>>();

        std::thread::spawn(move || {
            let timeout = Duration::from_secs(10);
            let poll_every = Duration::from_millis(250);
            let start = Instant::now();

            while !PostgresDependency::is_ready_once(&conn_str) {
                if start.elapsed() >= timeout {
                    let _ = tx.send(Err(format!(
                        "[PostgresDependency-{}] postgres did not become ready within {:?}. conn_str={:?}",
                        identifier,
                        timeout,
                        conn_str
                    )));
                    return;
                }

                std::thread::sleep(poll_every);
            }

            if let Some(scripts) = scripts {
                PostgresDependency::run_startup_sql_scripts(&identifier, &conn_str, &scripts);
            }

            let _ = tx.send(Ok(()));
        });

        match rx.await {
            Ok(Ok(())) => {}
            Ok(Err(msg)) => panic!("{msg}"),
            Err(_canceled) => panic!(
                "[PostgresDependency-{}] readiness/scripts worker thread unexpectedly stopped.",
                self.identifier
            ),
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