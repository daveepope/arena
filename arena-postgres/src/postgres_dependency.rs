mod healthcheck;
pub mod postgres_container_impl;

use arena::lifecycle::message;
use arena::lifecycle::Subject;
use crate::blocking::run_blocking;
use crate::builder::PostgresDependencyBuilder;
use crate::playbook::Playbook;
use crate::postgres_dependency::healthcheck::DefaultPostgresReadinessCheck;
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena::lifecycle::{Fault, RunnableState};
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
    state: RunnableState,
    faults: Vec<Fault>,
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
            state: RunnableState::NotStarted,
            faults: Vec::new(),
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

    fn run_startup_sql_scripts(
        identifier: &str,
        conn_str: &str,
        scripts: &[String],
    ) -> Result<(), String> {
        let mut client = postgres::Client::connect(conn_str, postgres::NoTls)
            .map_err(|err| format!("connect to run startup sql scripts failed: {err}"))?;

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

            client.batch_execute(sql).map_err(|err| {
                format!(
                    "startup sql script {}/{} failed: {err}",
                    idx + 1,
                    scripts.len()
                )
            })?;
        }

        tracing::debug!(dependency = %identifier, "startup sql scripts complete");
        Ok(())
    }

    async fn wait_until_ready(&self) -> Result<(), String> {
        let conn_str = self
            .connection_string()
            .ok_or("connection string not available after postgres started")?;

        self.readiness_check
            .is_ready(&self.identifier, conn_str, READINESS_TIMEOUT.as_millis() as u64)
            .await
            .map_err(message::readiness_failed)
    }

    async fn run_startup_sql_scripts_blocking(&self, scripts: Vec<String>) -> Result<(), String> {
        let identifier = self.identifier.clone();
        let conn_str = self
            .connection_string()
            .ok_or("connection string not available after postgres started")?
            .to_string();

        run_blocking(move || {
            PostgresDependency::run_startup_sql_scripts(&identifier, &conn_str, &scripts)
        })
        .await
    }

    async fn fail(&mut self, message: impl Into<String>, causes: Vec<Fault>) -> Fault {
        let fault = Fault::dependency(&self.identifier, message).caused_by_all(causes);
        self.faults.push(fault.clone());
        <Self as RunnableDependency>::force_stop(self).await;
        fault
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

    fn state(&self) -> RunnableState {
        self.state
    }

    fn faults(&self) -> &[Fault] {
        &self.faults
    }

    async fn start(&mut self) -> Result<(), Fault> {
        if self.running {
            return Ok(());
        }
        self.state = RunnableState::Starting;

        tracing::debug!(dependency = %self.identifier, phase = "start_begin", "starting");
        let sw = Instant::now();

        if let Some(children) = self.dependencies.as_mut() {
            if !children.is_empty() {
                self.children_started = true;
                let mut child_faults = Vec::new();
                for dep in children.iter_mut() {
                    if let Err(fault) = arena::dependency::start_child(dep).await {
                        child_faults.push(fault);
                    }
                }
                if !child_faults.is_empty() {
                    return Err(self.fail(message::child_start_failed(Subject::Dependency), child_faults).await);
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
        if let Err(message) = self
            .postgres_impl
            .start(
                self.port,
                &database_name,
                &database_username,
                &database_password,
                &image_name,
                &image_tag,
                &container_name,
            )
            .await
        {
            return Err(self.fail(message, Vec::new()).await);
        }
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_container.elapsed(),
            "container start finished"
        );

        let sw_ready = Instant::now();
        self.state = RunnableState::ReadinessCheck;
        if let Err(message) = self.wait_until_ready().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_ready.elapsed(),
            "readiness wait finished"
        );

        if let Some(scripts) = scripts {
            let sw_scripts = Instant::now();
            if let Err(message) = self.run_startup_sql_scripts_blocking(scripts).await {
                return Err(self.fail(message, Vec::new()).await);
            }
            tracing::debug!(
                dependency = %self.identifier,
                elapsed = ?sw_scripts.elapsed(),
                "startup scripts finished"
            );
        }

        self.running = true;
        self.state = RunnableState::Started;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "started and ready"
        );
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        self.state = RunnableState::Stopping;
        let mut causes = Vec::new();

        if let Err(message) = self.postgres_impl.stop().await {
            causes.push(Fault::dependency(&self.identifier, message));
        }
        self.needs_teardown = false;

        tracing::debug!(dependency = %self.identifier, phase = "stop_begin", "stopping");
        let sw = Instant::now();

        for dep in self.dependencies.iter_mut().flatten().rev() {
            if let Err(fault) = arena::dependency::stop_child(dep).await {
                causes.push(fault);
            }
        }

        self.children_started = false;
        self.running = false;

        if !causes.is_empty() {
            let fault =
                Fault::dependency(&self.identifier, message::stop_did_not_complete()).caused_by_all(causes);
            self.faults.push(fault.clone());
            self.state = RunnableState::Faulted;
            return Err(fault);
        }

        self.state = RunnableState::Stopped;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "stopped"
        );
        Ok(())
    }

    fn release(&mut self) {
        self.postgres_impl.release();
        self.running = false;
        self.needs_teardown = false;
        self.children_started = false;
        for dep in self.dependencies.iter_mut().flatten().rev() {
            arena::dependency::release_child(dep);
        }
        self.state = RunnableState::Stopped;
    }

    async fn force_stop(&mut self) {
        let removed = self.postgres_impl.force_stop().await;
        self.needs_teardown = false;
        self.running = false;
        self.children_started = false;

        for dep in self.dependencies.iter_mut().flatten().rev() {
            arena::dependency::force_stop_child(dep).await;
        }

        if removed {
            self.state = RunnableState::Stopped;
            return;
        }

        let unconfirmed = Fault::dependency(
            &self.identifier,
            message::forced_teardown_unconfirmed(),
        );
        if !self
            .faults
            .iter()
            .any(|f| f.message == unconfirmed.message && f.id == unconfirmed.id)
        {
            self.faults.push(unconfirmed);
        }
        self.state = RunnableState::Faulted;
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

    async fn soft_reset(&self) -> Result<(), Fault> {
        if !self.running {
            return Ok(());
        }

        let Some(scripts) = &self.startup_sql_scripts else {
            tracing::warn!(
                dependency = %self.identifier,
                "soft reset skipped: no startup scripts"
            );
            return Ok(());
        };

        let conn_str = self.connection_string().ok_or_else(|| {
            Fault::dependency(&self.identifier, "connection string not available for soft reset")
        })?;

        tracing::debug!(
            dependency = %self.identifier,
            phase = "soft_reset",
            "drop and recreate schema"
        );
        let mut client = postgres::Client::connect(conn_str, postgres::NoTls).map_err(|err| {
            Fault::dependency(&self.identifier, format!("connect for soft reset failed: {err}"))
        })?;
        client
            .batch_execute("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
            .map_err(|err| {
                Fault::dependency(
                    &self.identifier,
                    format!("drop and recreate schema failed: {err}"),
                )
            })?;
        drop(client);

        Self::run_startup_sql_scripts(&self.identifier, conn_str, scripts)
            .map_err(|message| Fault::dependency(&self.identifier, message))
    }

    async fn hard_reset(&mut self) -> Result<(), Fault> {
        if !self.running {
            return Ok(());
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

        if let Err(message) = self.postgres_impl.stop().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        self.running = false;

        if let Err(message) = self
            .postgres_impl
            .start(
                self.port,
                &database_name,
                &database_username,
                &database_password,
                &image_name,
                &image_tag,
                &container_name,
            )
            .await
        {
            return Err(self.fail(message, Vec::new()).await);
        }
        if let Err(message) = self.wait_until_ready().await {
            return Err(self.fail(message, Vec::new()).await);
        }

        if let Some(scripts) = scripts {
            if let Err(message) = self.run_startup_sql_scripts_blocking(scripts).await {
                return Err(self.fail(message, Vec::new()).await);
            }
        }

        self.running = true;
        self.state = RunnableState::Started;
        Ok(())
    }
}

impl Drop for PostgresDependency {
    fn drop(&mut self) {
        if self.running || self.needs_teardown || self.children_started {
            tracing::warn!(
                dependency = %self.identifier,
                "drop while running; releasing container"
            );
            self.postgres_impl.release();
            self.running = false;
            self.needs_teardown = false;
            self.children_started = false;
            self.state = RunnableState::Stopped;
        }
    }
}
