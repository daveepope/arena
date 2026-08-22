pub mod healthcheck;
pub mod oracle_container_impl;
pub mod sqlplus;

use crate::builder::OracleDependencyBuilder;
use crate::oracle_dependency::healthcheck::DefaultOracleReadinessCheck;
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use oracle_container_impl::OracleImpl;
use std::sync::Arc;
use std::time::Instant;

pub(crate) const ADMIN_USERNAME: &str = "system";
const READINESS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
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
    managed_tables: Vec<String>,
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
            managed_tables: Vec::new(),
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

    pub async fn execute(&self, sql: &str) {
        oracle_container_impl::exec_sql(
            self.oracle_impl.as_ref(),
            &self.database_username,
            &self.database_password,
            sql,
        )
        .await
        .unwrap_or_else(|e| panic!("[OracleDependency-{}] execute: {e}", self.identifier));
    }

    pub async fn query_scalar(&self, sql: &str) -> i32 {
        oracle_container_impl::exec_scalar_query(
            self.oracle_impl.as_ref(),
            &self.database_username,
            &self.database_password,
            sql,
        )
        .await
        .unwrap_or_else(|e| panic!("[OracleDependency-{}] query_scalar: {e}", self.identifier))
    }

    pub fn playbook(&self) -> crate::playbook::Playbook {
        crate::playbook::Playbook::with(self)
    }

    pub(crate) fn set_readiness_check(&mut self, check: Box<dyn ReadinessCheck>) {
        self.readiness_check = check;
    }

    async fn run_startup_sql_scripts(&self, scripts: &[String]) {
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
            .unwrap_or_else(|e| {
                panic!(
                    "[OracleDependency-{}] startup sql script {}/{} failed: {e}",
                    self.identifier,
                    idx + 1,
                    scripts.len()
                )
            });
        }

        tracing::debug!(dependency = %self.identifier, "startup sql scripts complete");
    }

    async fn wait_until_ready(&self) {
        let target = self
            .oracle_impl
            .host_address()
            .expect("host address should be available after oracle starts");

        let timeout_ms = READINESS_TIMEOUT.as_millis() as u64;

        match self
            .readiness_check
            .is_ready(&self.identifier, &target, timeout_ms)
            .await
        {
            Ok(()) => {}
            Err(msg) => panic!("{msg}"),
        }

        oracle_container_impl::exec_scalar_query(
            self.oracle_impl.as_ref(),
            &self.database_username,
            &self.database_password,
            SQL_READINESS_QUERY,
        )
        .await
        .unwrap_or_else(|e| {
            panic!(
                "[OracleDependency-{}] sql-level readiness check failed: {e}",
                self.identifier
            )
        });
    }

    async fn snapshot_managed_tables(&mut self) {
        let tables = oracle_container_impl::exec_table_list(
            self.oracle_impl.as_ref(),
            &self.database_username,
            &self.database_password,
            "SELECT TABLE_NAME FROM USER_TABLES ORDER BY TABLE_NAME;",
        )
        .await
        .unwrap_or_else(|e| panic!("[OracleDependency-{}] snapshot managed tables: {e}", self.identifier));

        tracing::debug!(
            dependency = %self.identifier,
            table_count = tables.len(),
            tables = ?tables,
            "captured managed table snapshot"
        );
        self.managed_tables = tables;
    }

    async fn recreate_app_user(&self) {
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
        .unwrap_or_else(|e| panic!("[OracleDependency-{}] soft reset: drop user: {e}", self.identifier));

        oracle_container_impl::exec_sql(
            self.oracle_impl.as_ref(),
            ADMIN_USERNAME,
            &self.admin_password,
            &create_sql,
        )
        .await
        .unwrap_or_else(|e| panic!("[OracleDependency-{}] soft reset: create user: {e}", self.identifier));
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
        self.oracle_impl
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

        if let Some(scripts) = self.startup_sql_scripts.clone() {
            let sw_scripts = Instant::now();
            self.run_startup_sql_scripts(&scripts).await;
            tracing::debug!(
                dependency = %self.identifier,
                elapsed = ?sw_scripts.elapsed(),
                "startup scripts finished"
            );
        }

        self.snapshot_managed_tables().await;

        self.running = true;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "started and ready"
        );
    }

    async fn stop(&mut self) {
        self.oracle_impl.stop().await;
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

        tracing::debug!(
            dependency = %self.identifier,
            phase = "soft_reset",
            "drop and recreate app user"
        );

        self.recreate_app_user().await;
        self.run_startup_sql_scripts(scripts).await;
    }

    async fn hard_reset(&mut self) {
        if !self.running {
            return;
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

        self.oracle_impl.stop().await;
        self.running = false;

        self.oracle_impl
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
            .await;
        self.wait_until_ready().await;

        if let Some(scripts) = self.startup_sql_scripts.clone() {
            self.run_startup_sql_scripts(&scripts).await;
        }
        self.snapshot_managed_tables().await;

        self.running = true;
    }
}

impl Drop for OracleDependency {
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
