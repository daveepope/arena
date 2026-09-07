pub mod healthcheck;
pub mod oracle_container_impl;
pub mod sqlplus;

use arena::lifecycle::message;
use arena::lifecycle::Subject;
use crate::builder::OracleDependencyBuilder;
use crate::oracle_dependency::healthcheck::DefaultOracleReadinessCheck;
use arena::dependency::{Dependency, RunnableDependency};
use arena::lifecycle::{Fault, RunnableState};
use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use oracle_container_impl::OracleImpl;
use std::sync::Arc;
use std::time::Instant;

pub(crate) const ADMIN_USERNAME: &str = "system";
const READINESS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
pub(crate) const FAST_SQL_READINESS_TIMEOUT: std::time::Duration = READINESS_TIMEOUT;
pub(crate) const FULL_BUILD_SQL_READINESS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
const SQL_READINESS_QUERY: &str = "SELECT 1 FROM DUAL";

pub struct OracleDependency {
    pub identifier: String,
    oracle_impl: Arc<dyn OracleImpl>,
    port: u16,
    database_name: String,
    database_username: String,
    database_password: String,
    admin_password: String,
    startup_sql_scripts: Option<Vec<String>>,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    running: bool,
    needs_teardown: bool,
    children_started: bool,
    image_name: String,
    image_tag: String,
    container_name: Option<String>,
    readiness_check: Box<dyn ReadinessCheck>,
    sql_readiness_timeout: std::time::Duration,
    managed_tables: Vec<String>,
    state: RunnableState,
    faults: Vec<Fault>,
}

impl OracleDependency {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        identifier: String,
        oracle_impl: Arc<dyn OracleImpl>,
        port: u16,
        database_name: String,
        database_username: String,
        database_password: String,
        admin_password: String,
        startup_sql_scripts: Option<Vec<String>>,
        dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
        image_name: String,
        image_tag: String,
        container_name: Option<String>,
    ) -> Self {
        Self {
            identifier,
            oracle_impl,
            port,
            database_name,
            database_username,
            database_password,
            admin_password,
            startup_sql_scripts,
            dependencies,
            image_name,
            image_tag,
            container_name,
            running: false,
            needs_teardown: false,
            children_started: false,
            readiness_check: Box::new(DefaultOracleReadinessCheck::new()),
            sql_readiness_timeout: READINESS_TIMEOUT,
            managed_tables: Vec::new(),
            state: RunnableState::NotStarted,
            faults: Vec::new(),
        }
    }

    pub fn connection_string(&self) -> Option<String> {
        self.oracle_impl.connection_string()
    }

    pub fn database_name(&self) -> &str {
        &self.database_name
    }

    pub fn database_username(&self) -> &str {
        &self.database_username
    }

    pub(crate) fn database_password(&self) -> &str {
        &self.database_password
    }

    pub(crate) fn oracle_impl(&self) -> Arc<dyn OracleImpl> {
        Arc::clone(&self.oracle_impl)
    }

    pub fn managed_tables(&self) -> &[String] {
        &self.managed_tables
    }

    pub fn builder(identifier: impl Into<String>) -> OracleDependencyBuilder {
        OracleDependencyBuilder::new(identifier)
    }

    pub async fn execute(&self, sql: &str) -> Result<(), Fault> {
        oracle_container_impl::exec_sql(
            self.oracle_impl.as_ref(),
            &self.database_username,
            &self.database_password,
            sql,
        )
        .await
        .map(|_| ())
        .map_err(|e| Fault::dependency(&self.identifier, format!("execute: {e}")))
    }

    pub async fn query_scalar(&self, sql: &str) -> Result<i32, Fault> {
        oracle_container_impl::exec_scalar_query(
            self.oracle_impl.as_ref(),
            &self.database_username,
            &self.database_password,
            sql,
        )
        .await
        .map_err(|e| Fault::dependency(&self.identifier, format!("query_scalar: {e}")))
    }

    pub fn playbook(&self) -> crate::playbook::Playbook {
        crate::playbook::Playbook::with(self)
    }

    pub(crate) fn set_readiness_check(&mut self, check: Box<dyn ReadinessCheck>) {
        self.readiness_check = check;
    }

    pub(crate) fn set_sql_readiness_timeout(&mut self, timeout: std::time::Duration) {
        self.sql_readiness_timeout = timeout;
    }

    async fn run_startup_sql_scripts(&self, scripts: &[String]) -> Result<(), String> {
        tracing::debug!(
            dependency = %self.identifier,
            script_count = scripts.len(),
            "running startup sql scripts"
        );

        for (idx, sql) in scripts.iter().enumerate() {
            tracing::debug!(
                dependency = %self.identifier,
                script_index = idx + 1,
                script_total = scripts.len(),
                "executing startup sql script"
            );

            oracle_container_impl::exec_sql(
                self.oracle_impl.as_ref(),
                &self.database_username,
                &self.database_password,
                sql,
            )
            .await
            .map_err(|e| {
                format!(
                    "startup sql script {}/{} failed: {e}",
                    idx + 1,
                    scripts.len()
                )
            })?;
        }

        tracing::debug!(dependency = %self.identifier, "startup sql scripts complete");
        Ok(())
    }

    async fn fail(&mut self, message: impl Into<String>, causes: Vec<Fault>) -> Fault {
        let fault = Fault::dependency(&self.identifier, message).caused_by_all(causes);
        self.faults.push(fault.clone());
        <Self as RunnableDependency>::force_stop(self).await;
        fault
    }

    async fn wait_until_ready(&self) -> Result<(), String> {
        let target = self
            .oracle_impl
            .host_address()
            .ok_or("host address not available after oracle started")?;

        let timeout_ms = READINESS_TIMEOUT.as_millis() as u64;

        self.readiness_check
            .is_ready(&self.identifier, &target, timeout_ms)
            .await
            .map_err(message::readiness_failed)?;

        self.wait_for_sql_ready().await
    }

    async fn wait_for_sql_ready(&self) -> Result<(), String> {
        let start = std::time::Instant::now();
        let poll_every = std::time::Duration::from_millis(200);

        loop {
            match oracle_container_impl::exec_scalar_query(
                self.oracle_impl.as_ref(),
                &self.database_username,
                &self.database_password,
                SQL_READINESS_QUERY,
            )
            .await
            {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if !self.oracle_impl.is_container_running().await {
                        return Err(format!(
                            "container stopped or was removed during sql-level readiness after {:?}: {e}",
                            start.elapsed()
                        ));
                    }

                    if start.elapsed() >= self.sql_readiness_timeout {
                        return Err(format!(
                            "sql-level readiness check did not succeed within {:?}: {e}",
                            self.sql_readiness_timeout
                        ));
                    }

                    tracing::debug!(
                        dependency = %self.identifier,
                        error = %e,
                        "sql-level readiness probe failed (will retry)"
                    );
                    futures_timer::Delay::new(poll_every).await;
                }
            }
        }
    }

    async fn snapshot_managed_tables(&mut self) -> Result<(), String> {
        let tables = oracle_container_impl::exec_table_list(
            self.oracle_impl.as_ref(),
            &self.database_username,
            &self.database_password,
            "SELECT TABLE_NAME FROM USER_TABLES ORDER BY TABLE_NAME;",
        )
        .await
        .map_err(|e| format!("snapshot managed tables: {e}"))?;

        tracing::debug!(
            dependency = %self.identifier,
            table_count = tables.len(),
            tables = ?tables,
            "captured managed table snapshot"
        );
        self.managed_tables = tables;
        Ok(())
    }

    async fn recreate_app_user(&self) -> Result<(), String> {
        let safe_user = self.database_username.replace('"', "\"\"");
        let safe_password = self.database_password.replace('\'', "''");

        let drop_sql = format!(
            "BEGIN\n\
             EXECUTE IMMEDIATE 'DROP USER \"{safe_user}\" CASCADE';\n\
             EXCEPTION WHEN OTHERS THEN\n\
             IF SQLCODE != -1918 THEN RAISE; END IF;\n\
             END;\n/"
        );
        let create_sql = format!(
            "CREATE USER \"{safe_user}\" IDENTIFIED BY \"{safe_password}\";\n\
             GRANT CONNECT, RESOURCE, UNLIMITED TABLESPACE TO \"{safe_user}\";"
        );

        oracle_container_impl::exec_sql(
            self.oracle_impl.as_ref(),
            ADMIN_USERNAME,
            &self.admin_password,
            &drop_sql,
        )
        .await
        .map_err(|e| format!("soft reset: drop user: {e}"))?;

        oracle_container_impl::exec_sql(
            self.oracle_impl.as_ref(),
            ADMIN_USERNAME,
            &self.admin_password,
            &create_sql,
        )
        .await
        .map_err(|e| format!("soft reset: create user: {e}"))?;
        Ok(())
    }
}

#[async_trait]
impl RunnableDependency for OracleDependency {
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

        let database_name = self.database_name.clone();
        let database_username = self.database_username.clone();
        let database_password = self.database_password.clone();
        let admin_password = self.admin_password.clone();
        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = arena_container::identifier::resolve_container_name(
            &self.identifier,
            self.container_name.as_deref(),
        );

        let sw_container = Instant::now();
        self.needs_teardown = true;
        if let Err(message) = self
            .oracle_impl
            .start(
                self.port,
                &database_name,
                &database_username,
                &database_password,
                &admin_password,
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

        if let Some(scripts) = self.startup_sql_scripts.clone() {
            let sw_scripts = Instant::now();
            if let Err(message) = self.run_startup_sql_scripts(&scripts).await {
                return Err(self.fail(message, Vec::new()).await);
            }
            tracing::debug!(
                dependency = %self.identifier,
                elapsed = ?sw_scripts.elapsed(),
                "startup scripts finished"
            );
        }

        if let Err(message) = self.snapshot_managed_tables().await {
            return Err(self.fail(message, Vec::new()).await);
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

        if let Err(message) = self.oracle_impl.stop().await {
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
        self.oracle_impl.release();
        self.running = false;
        self.needs_teardown = false;
        self.children_started = false;
        for dep in self.dependencies.iter_mut().flatten().rev() {
            arena::dependency::release_child(dep);
        }
        self.state = RunnableState::Stopped;
    }

    async fn force_stop(&mut self) {
        let removed = self.oracle_impl.force_stop().await;
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

        tracing::debug!(
            dependency = %self.identifier,
            phase = "soft_reset",
            "drop and recreate app user"
        );

        self.recreate_app_user()
            .await
            .map_err(|message| Fault::dependency(&self.identifier, message))?;
        self.run_startup_sql_scripts(scripts)
            .await
            .map_err(|message| Fault::dependency(&self.identifier, message))
    }

    async fn hard_reset(&mut self) -> Result<(), Fault> {
        if !self.running {
            return Ok(());
        }

        tracing::debug!(
            dependency = %self.identifier,
            phase = "hard_reset",
            "restarting oracle container"
        );

        let database_name = self.database_name.clone();
        let database_username = self.database_username.clone();
        let database_password = self.database_password.clone();
        let admin_password = self.admin_password.clone();
        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = arena_container::identifier::resolve_container_name(
            &self.identifier,
            self.container_name.as_deref(),
        );

        if let Err(message) = self.oracle_impl.stop().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        self.running = false;

        if let Err(message) = self
            .oracle_impl
            .start(
                self.port,
                &database_name,
                &database_username,
                &database_password,
                &admin_password,
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

        if let Some(scripts) = self.startup_sql_scripts.clone() {
            if let Err(message) = self.run_startup_sql_scripts(&scripts).await {
                return Err(self.fail(message, Vec::new()).await);
            }
        }
        if let Err(message) = self.snapshot_managed_tables().await {
            return Err(self.fail(message, Vec::new()).await);
        }

        self.running = true;
        self.state = RunnableState::Started;
        Ok(())
    }
}

impl Drop for OracleDependency {
    fn drop(&mut self) {
        if self.running || self.needs_teardown || self.children_started {
            tracing::warn!(
                dependency = %self.identifier,
                "drop while running; releasing container"
            );
            self.oracle_impl.release();
            self.running = false;
            self.needs_teardown = false;
            self.children_started = false;
            self.state = RunnableState::Stopped;
        }
    }
}
