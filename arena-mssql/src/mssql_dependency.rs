pub mod healthcheck;
pub mod mssql_container_impl;

use crate::builder::MssqlDependencyBuilder;
use crate::mssql_dependency::healthcheck::DefaultMssqlReadinessCheck;
use crate::mssql_dependency::mssql_container_impl::DEFAULT_CONNECT_TIMEOUT;
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use futures::channel::oneshot;
use mssql_container_impl::MssqlImpl;
use std::time::{Duration, Instant};

const READINESS_TIMEOUT: Duration = Duration::from_secs(30);

pub struct MssqlDependency {
    pub identifier: String,
    mssql_impl: Box<dyn MssqlImpl>,
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
    connect_timeout: Option<Duration>,
    managed_tables: Vec<(String, String)>,
}

impl MssqlDependency {
    pub(crate) fn new(
        identifier: String,
        mssql_impl: Box<dyn MssqlImpl>,
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
            mssql_impl,
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
            readiness_check: Box::new(DefaultMssqlReadinessCheck::new()),
            connect_timeout: Some(DEFAULT_CONNECT_TIMEOUT),
            managed_tables: Vec::new(),
        }
    }

    pub fn connection_string(&self) -> Option<&str> {
        self.mssql_impl.connection_string()
    }

    pub fn admin_connection_string(&self) -> Option<&str> {
        self.mssql_impl.admin_connection_string()
    }

    pub fn database_name(&self) -> &str {
        &self.database_name
    }

    pub fn managed_tables(&self) -> &[(String, String)] {
        &self.managed_tables
    }

    pub fn builder(identifier: impl Into<String>) -> MssqlDependencyBuilder {
        MssqlDependencyBuilder::new(identifier)
    }

    pub fn playbook(&self) -> crate::playbook::Playbook {
        crate::playbook::Playbook::with(self)
    }

    pub(crate) fn set_readiness_check(&mut self, check: Box<dyn ReadinessCheck>) {
        self.readiness_check = check;
    }

    pub(crate) fn set_connect_timeout(&mut self, connect_timeout: Option<Duration>) {
        self.connect_timeout = connect_timeout;
    }

    pub fn connect_timeout(&self) -> Option<Duration> {
        self.connect_timeout
    }

    async fn run_startup_sql_scripts(
        identifier: &str,
        conn_str: &str,
        scripts: &[String],
        connect_timeout: Option<Duration>,
    ) -> Result<(), String> {
        let mut client = mssql_container_impl::connect_with_timeout(conn_str, connect_timeout)
            .await
            .map_err(|e| format!("[MssqlDependency-{identifier}] connect for startup scripts: {e}"))?;

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

            client.simple_query(sql.as_str()).await.map_err(|err| {
                format!(
                    "[MssqlDependency-{}] startup sql script {}/{} failed: {err}",
                    identifier,
                    idx + 1,
                    scripts.len()
                )
            })?;
        }

        tracing::debug!(dependency = %identifier, "startup sql scripts complete");
        Ok(())
    }

    async fn wait_until_ready(&self) {
        let conn_str = self
            .admin_connection_string()
            .expect("admin connection string should be available after mssql starts");

        match self
            .readiness_check
            .is_ready(&self.identifier, conn_str, READINESS_TIMEOUT.as_millis() as u64)
            .await
        {
            Ok(()) => {}
            Err(msg) => panic!("{msg}"),
        }
    }

    async fn ensure_database_exists(&self) {
        if self.database_name.eq_ignore_ascii_case("master") {
            return;
        }

        let admin_conn = self
            .admin_connection_string()
            .expect("admin connection string should be available after mssql starts")
            .to_string();
        let identifier = self.identifier.clone();
        let database_name = self.database_name.clone();
        let connect_timeout = self.connect_timeout;

        let (tx, rx) = oneshot::channel::<Result<(), String>>();

        tokio::spawn(async move {
            let res = async {
                let mut client =
                    mssql_container_impl::connect_with_timeout(&admin_conn, connect_timeout)
                        .await
                        .map_err(|e| format!("connect to master: {e}"))?;

                let safe = database_name.replace(']', "]]");
                let sql = format!(
                    "IF DB_ID(N'{name}') IS NULL CREATE DATABASE [{safe}];",
                    name = database_name.replace('\'', "''"),
                    safe = safe,
                );

                client
                    .simple_query(sql.as_str())
                    .await
                    .map_err(|e| format!("create database [{database_name}] failed: {e}"))?;

                tracing::debug!(
                    dependency = %identifier,
                    database = %database_name,
                    "ensured database exists"
                );
                Ok::<(), String>(())
            }
            .await;
            let _ = tx.send(res);
        });

        match rx.await {
            Ok(Ok(())) => {}
            Ok(Err(msg)) => panic!(
                "[MssqlDependency-{}] ensure_database_exists: {msg}",
                self.identifier
            ),
            Err(_canceled) => panic!(
                "[MssqlDependency-{}] ensure_database_exists worker unexpectedly stopped.",
                self.identifier
            ),
        }
    }

    async fn snapshot_managed_tables(&mut self) {
        let identifier = self.identifier.clone();
        let conn_str = self
            .connection_string()
            .expect("connection string should be available after mssql starts")
            .to_string();
        let connect_timeout = self.connect_timeout;

        let (tx, rx) = oneshot::channel::<Result<Vec<(String, String)>, String>>();

        tokio::spawn(async move {
            let res = async {
                let mut client =
                    mssql_container_impl::connect_with_timeout(&conn_str, connect_timeout)
                        .await
                        .map_err(|e| format!("connect to snapshot tables: {e}"))?;

                let sql = "SELECT s.name AS schema_name, t.name AS table_name \
                           FROM sys.tables t \
                           INNER JOIN sys.schemas s ON t.schema_id = s.schema_id \
                           WHERE t.is_ms_shipped = 0 \
                           ORDER BY s.name, t.name;";

                let stream = client
                    .simple_query(sql)
                    .await
                    .map_err(|e| format!("snapshot tables query failed: {e}"))?;

                let rows = stream
                    .into_first_result()
                    .await
                    .map_err(|e| format!("snapshot tables read failed: {e}"))?;

                let mut tables = Vec::with_capacity(rows.len());
                for row in rows {
                    let schema: &str = row.get::<&str, _>(0).unwrap_or("");
                    let name: &str = row.get::<&str, _>(1).unwrap_or("");
                    if !schema.is_empty() && !name.is_empty() {
                        tables.push((schema.to_string(), name.to_string()));
                    }
                }
                Ok::<Vec<(String, String)>, String>(tables)
            }
            .await;
            let _ = tx.send(res);
        });

        match rx.await {
            Ok(Ok(tables)) => {
                tracing::debug!(
                    dependency = %identifier,
                    table_count = tables.len(),
                    tables = ?tables,
                    "captured managed table snapshot"
                );
                self.managed_tables = tables;
            }
            Ok(Err(msg)) => panic!("[MssqlDependency-{identifier}] snapshot managed tables: {msg}"),
            Err(_canceled) => panic!(
                "[MssqlDependency-{identifier}] snapshot managed tables worker unexpectedly stopped."
            ),
        }
    }

    async fn run_startup_sql_scripts_blocking(&self, scripts: Vec<String>) {
        let identifier = self.identifier.clone();
        let conn_str = self
            .connection_string()
            .expect("connection string should be available after mssql starts")
            .to_string();
        let connect_timeout = self.connect_timeout;

        let (tx, rx) = oneshot::channel::<Result<(), String>>();

        tokio::spawn(async move {
            let res = MssqlDependency::run_startup_sql_scripts(
                &identifier,
                &conn_str,
                &scripts,
                connect_timeout,
            )
            .await;
            let _ = tx.send(res);
        });

        match rx.await {
            Ok(Ok(())) => {}
            Ok(Err(msg)) => panic!("{msg}"),
            Err(_canceled) => panic!(
                "[MssqlDependency-{}] startup-scripts worker unexpectedly stopped.",
                self.identifier
            ),
        }
    }
}

#[async_trait]
impl RunnableDependency for MssqlDependency {
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
        self.mssql_impl
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

        self.ensure_database_exists().await;

        if let Some(scripts) = scripts {
            let sw_scripts = Instant::now();
            self.run_startup_sql_scripts_blocking(scripts).await;
            tracing::debug!(
                dependency = %self.identifier,
                elapsed = ?sw_scripts.elapsed(),
                "startup scripts finished"
            );

            self.snapshot_managed_tables().await;
        }

        self.running = true;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "started and ready"
        );
    }

    async fn stop(&mut self) {
        self.mssql_impl.stop().await;
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

        let admin_conn = self
            .admin_connection_string()
            .expect("admin connection string for soft reset")
            .to_string();
        let conn_str = self
            .connection_string()
            .expect("connection string for soft reset")
            .to_string();
        let database_name = self.database_name.clone();
        let identifier = self.identifier.clone();

        tracing::debug!(
            dependency = %self.identifier,
            phase = "soft_reset",
            "drop and recreate database"
        );

        let connect_timeout = self.connect_timeout;
        let reset_res: Result<(), String> = async {
            let mut admin = mssql_container_impl::connect_with_timeout(&admin_conn, connect_timeout)
                .await
                .map_err(|e| format!("connect to master: {e}"))?;

            let safe = database_name.replace(']', "]]");
            let drop_sql = format!(
                "IF DB_ID(N'{name}') IS NOT NULL BEGIN \
                 ALTER DATABASE [{safe}] SET SINGLE_USER WITH ROLLBACK IMMEDIATE; \
                 DROP DATABASE [{safe}]; \
                 END; \
                 CREATE DATABASE [{safe}];",
                name = database_name.replace('\'', "''"),
                safe = safe,
            );
            admin
                .simple_query(drop_sql.as_str())
                .await
                .map_err(|e| format!("drop/recreate database: {e}"))?;
            Ok(())
        }
        .await;

        if let Err(msg) = reset_res {
            panic!("[MssqlDependency-{identifier}] soft reset failed: {msg}");
        }

        if let Err(msg) =
            Self::run_startup_sql_scripts(&identifier, &conn_str, scripts, connect_timeout).await
        {
            panic!("{msg}");
        }
    }

    async fn hard_reset(&mut self) {
        if !self.running {
            return;
        }

        tracing::debug!(
            dependency = %self.identifier,
            phase = "hard_reset",
            "restarting mssql container"
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

        self.mssql_impl.stop().await;
        self.running = false;

        self.mssql_impl
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
        self.ensure_database_exists().await;

        if let Some(scripts) = scripts {
            self.run_startup_sql_scripts_blocking(scripts).await;
            self.snapshot_managed_tables().await;
        }

        self.running = true;
    }
}

impl Drop for MssqlDependency {
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
