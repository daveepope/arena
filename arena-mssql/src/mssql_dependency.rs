mod healthcheck;
pub mod mssql_container_impl;

use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use crate::builder::MssqlDependencyBuilder;
use mssql_container_impl::MssqlImpl;
use futures::channel::oneshot;
use std::time::{Duration, Instant};
use crate::mssql_dependency::healthcheck::DefaultMssqlReadinessCheck;

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
    image_name: String,
    image_tag: String,
    container_name: Option<String>,
    readiness_check: Box<dyn ReadinessCheck>,
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
            readiness_check: Box::new(DefaultMssqlReadinessCheck),
            managed_tables: Vec::new(),
        }
    }

    fn default_container_name(&self) -> String {
        arena_container::identifier::sanitize_for_container(&self.identifier)
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

    async fn run_startup_sql_scripts(
        identifier: &str,
        conn_str: &str,
        scripts: &[String],
    ) -> Result<(), String> {
        let mut client = mssql_container_impl::connect(conn_str)
            .await
            .map_err(|e| format!("[MssqlDependency-{identifier}] connect for startup scripts: {e}"))?;

        log::info!(
            "[MssqlDependency-{}] running {} startup sql script(s).",
            identifier,
            scripts.len()
        );

        for (idx, sql) in scripts.iter().enumerate() {
            log::info!(
                "[MssqlDependency-{}] running startup sql script {}/{}.",
                identifier,
                idx + 1,
                scripts.len()
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

        log::info!(
            "[MssqlDependency-{}] startup sql scripts complete.",
            identifier
        );
        Ok(())
    }

    async fn wait_until_ready(&self) {
        let conn_str = self
            .admin_connection_string()
            .expect("admin connection string should be available after mssql starts");

        let timeout = Duration::from_secs(60);

        match self
            .readiness_check
            .is_ready(&self.identifier, conn_str, timeout.as_millis() as u64)
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

        let (tx, rx) = oneshot::channel::<Result<(), String>>();

        tokio::spawn(async move {
            let res = async {
                let mut client = mssql_container_impl::connect(&admin_conn)
                    .await
                    .map_err(|e| format!("connect to master: {e}"))?;

                let safe = database_name.replace(']', "]]");
                let sql = format!(
                    "IF DB_ID(N'{name}') IS NULL CREATE DATABASE [{safe}];",
                    name = database_name.replace('\'', "''"),
                    safe = safe,
                );

                client.simple_query(sql.as_str()).await.map_err(|e| {
                    format!("create database [{database_name}] failed: {e}")
                })?;

                log::info!(
                    "[MssqlDependency-{identifier}] ensured database [{database_name}] exists"
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

        let (tx, rx) = oneshot::channel::<Result<Vec<(String, String)>, String>>();

        tokio::spawn(async move {
            let res = async {
                let mut client = mssql_container_impl::connect(&conn_str)
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
                log::info!(
                    "[MssqlDependency-{identifier}] snapshot {} managed table(s): {:?}",
                    tables.len(),
                    tables
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

        let (tx, rx) = oneshot::channel::<Result<(), String>>();

        tokio::spawn(async move {
            let res = MssqlDependency::run_startup_sql_scripts(&identifier, &conn_str, &scripts).await;
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

        log::info!("[MssqlDependency-{}] starting.", self.identifier);
        let sw = Instant::now();

        for dep in self.dependencies.iter_mut().flatten() {
            dep.start().await;
        }

        let scripts = self.startup_sql_scripts.clone();
        let database_name = self.database_name.clone();
        let database_username = self.database_username.clone();
        let database_password = self.database_password.clone();
        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = self
            .container_name
            .clone()
            .unwrap_or_else(|| self.default_container_name());

        let sw_container = Instant::now();
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
        log::debug!(
            "[MssqlDependency-{}] container start in {:?}.",
            self.identifier,
            sw_container.elapsed()
        );

        let sw_ready = Instant::now();
        self.wait_until_ready().await;
        log::debug!(
            "[MssqlDependency-{}] readiness in {:?}.",
            self.identifier,
            sw_ready.elapsed()
        );

        self.ensure_database_exists().await;

        if let Some(scripts) = scripts {
            let sw_scripts = Instant::now();
            self.run_startup_sql_scripts_blocking(scripts).await;
            log::debug!(
                "[MssqlDependency-{}] startup scripts in {:?}.",
                self.identifier,
                sw_scripts.elapsed()
            );

            self.snapshot_managed_tables().await;
        }

        self.running = true;
        log::debug!(
            "[MssqlDependency-{}] start complete in {:?}.",
            self.identifier,
            sw.elapsed()
        );
        log::info!("[MssqlDependency-{}] started and ready.", self.identifier);
    }

    async fn stop(&mut self) {
        if !self.running {
            return;
        }

        log::info!("[MssqlDependency-{}] stopping.", self.identifier);
        let sw = Instant::now();

        self.mssql_impl.stop().await;

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.stop().await;
        }

        self.running = false;
        log::debug!(
            "[MssqlDependency-{}] stop complete in {:?}.",
            self.identifier,
            sw.elapsed()
        );
        log::info!("[MssqlDependency-{}] stopped.", self.identifier);
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.get_or_insert_with(Vec::new).push(dep);
    }

    async fn soft_reset(&self) {
        if !self.running {
            return;
        }

        let Some(scripts) = &self.startup_sql_scripts else {
            log::warn!("[MssqlDependency-{}] soft reset: no startup scripts", self.identifier);
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

        log::info!("[MssqlDependency-{}] soft reset: dropping and recreating database", self.identifier);

        let reset_res: Result<(), String> = async {
            let mut admin = mssql_container_impl::connect(&admin_conn)
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
            Self::run_startup_sql_scripts(&identifier, &conn_str, scripts).await
        {
            panic!("{msg}");
        }
    }

    async fn hard_reset(&mut self) {
        if !self.running {
            return;
        }

        log::info!("[MssqlDependency-{}] hard reset: restarting container", self.identifier);

        let scripts = self.startup_sql_scripts.clone();
        let database_name = self.database_name.clone();
        let database_username = self.database_username.clone();
        let database_password = self.database_password.clone();
        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = self
            .container_name
            .clone()
            .unwrap_or_else(|| self.default_container_name());

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
        if !self.running {
            return;
        }
        log::warn!(
            "[MssqlDependency-{}] dropped while still running; stopping container.",
            self.identifier
        );
        futures::executor::block_on(<Self as RunnableDependency>::stop(self));
    }
}
