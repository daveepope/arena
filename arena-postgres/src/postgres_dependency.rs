use arena::dependency::RunnableDependency;
use async_trait::async_trait;
use crate::builder::PostgresDependencyBuilder;
use crate::postgres_container_impl::PostgresImpl;
use backon::{BlockingRetryable, ConstantBuilder};
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
    container_tag: String
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
        container_tag: String
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
            container_tag,
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

    async fn is_ready(&self) {
        let identifier = self.identifier.clone();
        let conn_str = self
            .connection_string()
            .expect("connection string should be available after postgres starts")
            .to_string();

        let conn_str_for_thread = conn_str.clone();
        let (tx, rx) = oneshot::channel::<Result<(), String>>();

        std::thread::spawn(move || {
            let timeout = Duration::from_secs(10);
            let poll_every = Duration::from_millis(250);
            let start = Instant::now();

            let policy = ConstantBuilder::default()
                .with_delay(poll_every)
                .without_max_times();

            let is_ready_once = || {
                if PostgresDependency::is_ready_once(&conn_str_for_thread) {
                    Ok(())
                } else {
                    Err(())
                }
            };

            let result = is_ready_once
                .retry(policy)
                .sleep(std::thread::sleep)
                // Preserve wall-clock timeout semantics (includes connect time).
                .when(|_| start.elapsed() < timeout)
                .call();

            match result {
                Ok(()) => {
                    let _ = tx.send(Ok(()));
                }
                Err(()) => {
                    let _ = tx.send(Err(format!(
                        "[PostgresDependency-{}] postgres did not become ready within {:?}. conn_str={:?}",
                        identifier, timeout, conn_str_for_thread
                    )));
                }
            }
        });

        match rx.await {
            Ok(Ok(())) => (),
            Ok(Err(msg)) => panic!("{msg}"),
            Err(_canceled) => panic!(
                "[PostgresDependency-{}] readiness/health-check worker thread unexpectedly stopped.",
                self.identifier
            ),
        }
    }

    async fn run_startup_sql_scripts_blocking(&self, scripts: Vec<String>) {
        // `postgres::Client::connect` can spin up a Tokio runtime internally, which will panic
        // if invoked from within an already-running Tokio runtime thread. To stay runtime-agnostic
        // and avoid that nested-runtime panic, run startup scripts on a dedicated OS thread.
        let identifier = self.identifier.clone();
        let conn_str = self
            .connection_string()
            .expect("connection string should be available after postgres starts")
            .to_string();

        let (tx, rx) = oneshot::channel::<Result<(), String>>();

        std::thread::spawn(move || {
            let res = std::panic::catch_unwind(|| {
                PostgresDependency::run_startup_sql_scripts(&identifier, &conn_str, &scripts);
            });

            match res {
                Ok(()) => {
                    let _ = tx.send(Ok(()));
                }
                Err(panic_payload) => {
                    let msg = if let Some(s) = panic_payload.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = panic_payload.downcast_ref::<&str>() {
                        s.to_string()
                    } else {
                        format!("[PostgresDependency-{}] startup sql scripts panicked.", identifier)
                    };

                    let _ = tx.send(Err(msg));
                }
            }
        });

        match rx.await {
            Ok(Ok(())) => {}
            Ok(Err(msg)) => panic!("{msg}"),
            Err(_canceled) => panic!(
                "[PostgresDependency-{}] startup-scripts worker thread unexpectedly stopped.",
                self.identifier
            ),
        }
    }
}

#[async_trait]
impl RunnableDependency for PostgresDependency {
    async fn start(&mut self) {
        if self.running {
            return;
        }

        log::info!("[PostgresDependency-{}] starting.", self.identifier);
        let sw = Instant::now();

        for dep in self.dependencies.iter_mut().flatten() {
            dep.start().await;
        }

        let scripts = self.startup_sql_scripts.take();
        let database_name = self.database_name.clone();
        let database_username = self.database_username.clone();
        let database_password = self.database_password.clone();
        let container_tag = self.container_tag.clone();

        let sw_container = Instant::now();
        self.postgres_impl
            .start(
                self.port,
                &database_name,
                &database_username,
                &database_password,
                &container_tag
            )
            .await;
        log::debug!(
            "[PostgresDependency-{}] container start in {:?}.",
            self.identifier,
            sw_container.elapsed()
        );

        let sw_ready = Instant::now();
        self.is_ready().await;
        log::debug!(
            "[PostgresDependency-{}] readiness in {:?}.",
            self.identifier,
            sw_ready.elapsed()
        );

        if let Some(scripts) = scripts {
            let sw_scripts = Instant::now();
            self.run_startup_sql_scripts_blocking(scripts).await;
            log::debug!(
                "[PostgresDependency-{}] startup scripts in {:?}.",
                self.identifier,
                sw_scripts.elapsed()
            );
        }

        self.running = true;
        log::debug!(
            "[PostgresDependency-{}] start complete in {:?}.",
            self.identifier,
            sw.elapsed()
        );
        log::info!("[PostgresDependency-{}] started and ready.", self.identifier);
    }

    async fn stop(&mut self) {
        if !self.running {
            return;
        }

        log::info!("[PostgresDependency-{}] stopping.", self.identifier);
        let sw = Instant::now();

        self.postgres_impl.stop().await;

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.stop().await;
        }

        self.running = false;
        log::debug!(
            "[PostgresDependency-{}] stop complete in {:?}.",
            self.identifier,
            sw.elapsed()
        );
        log::info!("[PostgresDependency-{}] stopped.", self.identifier);
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.get_or_insert_with(Vec::new).push(dep);
    }
}