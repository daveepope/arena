mod healthcheck;
pub mod postgres_container_impl;

use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use crate::builder::PostgresDependencyBuilder;
use postgres_container_impl::PostgresImpl;
use futures::channel::oneshot;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use crate::postgres_dependency::healthcheck::DefaultPostgresReadinessCheck;

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
    image_tag: String,
    container_name: Option<String>,
    readiness_check: Box<dyn ReadinessCheck>,
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
        image_tag: String,
        container_name: Option<String>,
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
            image_tag,
            container_name,
            running: false,
            readiness_check: Box::new(DefaultPostgresReadinessCheck),
        }
    }

    fn default_container_name(&self) -> String {
        let mut safe = String::with_capacity(self.identifier.len());
        for c in self.identifier.chars() {
            let c = c.to_ascii_lowercase();
            if c.is_ascii_alphanumeric() {
                safe.push(c);
            } else {
                safe.push('-');
            }
        }

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        format!("arena-postgres-{safe}-{ts}")
    }

    pub fn connection_string(&self) -> Option<&str> {
        self.postgres_impl.connection_string()
    }

    pub fn builder(identifier: impl Into<String>) -> PostgresDependencyBuilder {
        PostgresDependencyBuilder::new(identifier)
    }

    pub(crate) fn set_readiness_check(&mut self, check: Box<dyn ReadinessCheck>) {
        self.readiness_check = check;
    }

    fn run_startup_sql_scripts(identifier: &str, conn_str: &str, scripts: &[String]) {
        let mut client =
            postgres::Client::connect(conn_str, postgres::NoTls)
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

    async fn wait_until_ready(&self) {
        let conn_str = self
            .connection_string()
            .expect("connection string should be available after postgres starts");

        let timeout = Duration::from_secs(10);

        match self
            .readiness_check
            .is_ready(&self.identifier, conn_str, timeout.as_millis() as u64)
            .await
        {
            Ok(()) => {}
            Err(msg) => panic!("{msg}"),
        }
    }

    async fn run_startup_sql_scripts_blocking(&self, scripts: Vec<String>) {
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
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

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
        let image_tag = self.image_tag.clone();
        let container_name = self
            .container_name
            .clone()
            .unwrap_or_else(|| self.default_container_name());

        let sw_container = Instant::now();
        self.postgres_impl
            .start(
                self.port,
                &database_name,
                &database_username,
                &database_password,
                &image_tag,
                &container_name,
            )
            .await;
        log::debug!(
            "[PostgresDependency-{}] container start in {:?}.",
            self.identifier,
            sw_container.elapsed()
        );

        let sw_ready = Instant::now();
        self.wait_until_ready().await;
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