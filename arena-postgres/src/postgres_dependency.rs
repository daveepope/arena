mod healthcheck;
pub mod postgres_container_impl;

use crate::blocking::run_blocking;
use crate::builder::PostgresDependencyBuilder;
use crate::playbook::Playbook;
use crate::postgres_dependency::healthcheck::DefaultPostgresReadinessCheck;
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use postgres_container_impl::PostgresImpl;
use std::time::{Duration, Instant};

const READINESS_TIMEOUT: Duration = Duration::from_secs(30);

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
    needs_teardown: bool,
    children_started: bool,
    image_name: String,
    image_tag: String,
    container_name: Option<String>,
    readiness_check: Box<dyn ReadinessCheck>,
    managed_tables: Vec<(String, String)>,
}

impl PostgresDependency {
    pub(crate) fn new(
        identifier: String,
        postgres_impl: Box<dyn PostgresImpl>,
        port: u16,
        database_name: String,
        database_username: String,
        database_password: String,
        startup_sql_scripts: Option<Vec<String>>,
        dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
        image_name: String,
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
            image_name,
            image_tag,
            container_name,
            running: false,
            needs_teardown: false,
            children_started: false,
            readiness_check: Box::new(DefaultPostgresReadinessCheck),
            managed_tables: Vec::new(),
        }
    }

    pub fn connection_string(&self) -> Option<&str> {
        self.postgres_impl.connection_string()
    }

    pub fn managed_tables(&self) -> &[(String, String)] {
        &self.managed_tables
    }

    pub fn builder(identifier: impl Into<String>) -> PostgresDependencyBuilder {
        PostgresDependencyBuilder::new(identifier)
    }

    pub fn playbook(&self) -> Playbook {
        Playbook::with(self)
    }

    pub(crate) fn set_readiness_check(&mut self, check: Box<dyn ReadinessCheck>) {
        self.readiness_check = check;
    }

    fn run_startup_sql_scripts(identifier: &str, conn_str: &str, scripts: &[String]) {
        let mut client = postgres::Client::connect(conn_str, postgres::NoTls)
            .expect("connect to postgres to run startup scripts");

        tracing::debug!(
            dependency = %identifier,
            script_count = scripts.len(),
            "running startup sql scripts"
        );

        for (idx, sql) in scripts.iter().enumerate() {
            tracing::debug!(
                dependency = %identifier,
                script_index = idx + 1,
                script_total = scripts.len(),
                "executing startup sql script"
            );

            client.batch_execute(sql).unwrap_or_else(|err| {
                panic!(
                    "[PostgresDependency-{}] startup sql script {}/{} failed: {err}",
                    identifier,
                    idx + 1,
                    scripts.len()
                )
            });
        }

        tracing::debug!(dependency = %identifier, "startup sql scripts complete");
    }

    async fn wait_until_ready(&self) {
        let conn_str = self
            .connection_string()
            .expect("connection string should be available after postgres starts");

        match self
            .readiness_check
            .is_ready(&self.identifier, conn_str, READINESS_TIMEOUT.as_millis() as u64)
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

        run_blocking(move || {
            PostgresDependency::run_startup_sql_scripts(&identifier, &conn_str, &scripts);
        })
        .await;
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

        tracing::debug!(dependency = %self.identifier, phase = "start_begin", "starting");
        let sw = Instant::now();

        if let Some(children) = self.dependencies.as_mut() {
            if !children.is_empty() {
                self.children_started = true;
                for dep in children.iter_mut() {
                    dep.start().await;
                }
            }
        }

        let scripts = self.startup_sql_scripts.clone();
        let database_name = self.database_name.clone();
        let database_username = self.database_username.clone();
        let database_password = self.database_password.clone();
        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = arena_container::identifier::resolve_container_name(
            &self.identifier,
            self.container_name.as_deref(),
        );

        let sw_container = Instant::now();
        self.needs_teardown = true;
        self.postgres_impl
            .start(
                self.port,
                &database_name,
                &database_username,
                &database_password,
                &image_name,
                &image_tag,
                &container_name,
            )
            .await;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_container.elapsed(),
            "container start finished"
        );

        let sw_ready = Instant::now();
        self.wait_until_ready().await;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_ready.elapsed(),
            "readiness wait finished"
        );

        if let Some(scripts) = scripts {
            let sw_scripts = Instant::now();
            self.run_startup_sql_scripts_blocking(scripts).await;
            tracing::debug!(
                dependency = %self.identifier,
                elapsed = ?sw_scripts.elapsed(),
                "startup scripts finished"
            );
        }

        self.running = true;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "started and ready"
        );
    }

    async fn stop(&mut self) {
        self.postgres_impl.stop().await;
        self.needs_teardown = false;

        if !self.running {
            if self.children_started {
                for dep in self.dependencies.iter_mut().flatten().rev() {
                    dep.stop().await;
                }
                self.children_started = false;
            }
            return;
        }

        tracing::debug!(dependency = %self.identifier, phase = "stop_begin", "stopping");
        let sw = Instant::now();

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.stop().await;
        }

        self.children_started = false;
        self.running = false;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "stopped"
        );
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.get_or_insert_with(Vec::new).push(dep);
    }

    fn children(&self) -> &[Dependency] {
        self.dependencies.as_deref().unwrap_or(&[])
    }

    fn children_mut(&mut self) -> &mut [Dependency] {
        self.dependencies.as_deref_mut().unwrap_or(&mut [])
    }

    async fn soft_reset(&self) {
        if !self.running {
            return;
        }

        let Some(scripts) = &self.startup_sql_scripts else {
            tracing::warn!(
                dependency = %self.identifier,
                "soft reset skipped: no startup scripts"
            );
            return;
        };

        let conn_str = self
            .connection_string()
            .expect("connection string for soft reset");

        tracing::debug!(
            dependency = %self.identifier,
            phase = "soft_reset",
            "drop and recreate schema"
        );
        let mut client =
            postgres::Client::connect(conn_str, postgres::NoTls).expect("connect for soft reset");
        client
            .batch_execute("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
            .expect("drop/recreate schema");
        drop(client);

        Self::run_startup_sql_scripts(&self.identifier, conn_str, scripts);
    }

    async fn hard_reset(&mut self) {
        if !self.running {
            return;
        }

        tracing::debug!(
            dependency = %self.identifier,
            phase = "hard_reset",
            "restarting postgres container"
        );

        let scripts = self.startup_sql_scripts.clone();
        let database_name = self.database_name.clone();
        let database_username = self.database_username.clone();
        let database_password = self.database_password.clone();
        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = arena_container::identifier::resolve_container_name(
            &self.identifier,
            self.container_name.as_deref(),
        );

        self.postgres_impl.stop().await;
        self.running = false;

        self.postgres_impl
            .start(
                self.port,
                &database_name,
                &database_username,
                &database_password,
                &image_name,
                &image_tag,
                &container_name,
            )
            .await;
        self.wait_until_ready().await;

        if let Some(scripts) = scripts {
            self.run_startup_sql_scripts_blocking(scripts).await;
        }

        self.running = true;
    }
}

impl Drop for PostgresDependency {
    fn drop(&mut self) {
        if self.running {
            tracing::warn!(
                dependency = %self.identifier,
                "drop while running; forcing stop"
            );
            futures::executor::block_on(<Self as RunnableDependency>::stop(self));
        } else if self.needs_teardown || self.children_started {
            futures::executor::block_on(<Self as RunnableDependency>::stop(self));
        }
    }
}
