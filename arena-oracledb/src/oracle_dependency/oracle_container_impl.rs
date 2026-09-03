use crate::oracle_dependency::sqlplus;
use async_trait::async_trait;
use std::sync::Arc;
use testcontainers_modules::testcontainers::core::{ContainerPort, ExecCommand};
use testcontainers_modules::testcontainers::ImageExt;
use testcontainers_modules::{
    testcontainers, testcontainers::runners::AsyncRunner, testcontainers::GenericImage,
};
use tokio::sync::Mutex as AsyncMutex;

const DEFAULT_CONTAINER_PORT: u16 = 1521;
pub(crate) const DEFAULT_SERVICE_NAME: &str = "FREEPDB1";

#[async_trait]
pub trait OracleImpl: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn start(
        &self,
        port: u16,
        database_name: &str,
        database_username: &str,
        database_password: &str,
        admin_password: &str,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    );
    async fn stop(&self);

    fn connection_string(&self) -> Option<String>;

    fn host_address(&self) -> Option<String>;

    async fn run_sqlplus(
        &self,
        username: &str,
        password: &str,
        script: &str,
    ) -> Result<String, String>;

    async fn is_container_running(&self) -> bool {
        true
    }
}

#[derive(Default)]
struct OracleContainerMeta {
    connection_string: Option<String>,
    host_address: Option<String>,
    database_name: Option<String>,
}

pub(crate) struct OracleContainerImpl {
    container: AsyncMutex<Option<Arc<testcontainers::core::ContainerAsync<GenericImage>>>>,
    meta: std::sync::Mutex<OracleContainerMeta>,
    network: Option<String>,
}

impl OracleContainerImpl {
    pub(crate) fn new(network: Option<String>) -> Self {
        Self {
            container: AsyncMutex::new(None),
            meta: std::sync::Mutex::new(OracleContainerMeta::default()),
            network,
        }
    }
}

#[async_trait]
impl OracleImpl for OracleContainerImpl {
    async fn start(
        &self,
        port: u16,
        database_name: &str,
        database_username: &str,
        database_password: &str,
        admin_password: &str,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    ) {
        let mut container_guard = self.container.lock().await;
        if container_guard.is_some() {
            return;
        }

        arena_container::container::try_remove_existing_container(container_name).await;

        let container_port = ContainerPort::from(DEFAULT_CONTAINER_PORT);

        let image = GenericImage::new(image_name, image_tag).with_exposed_port(container_port);

        let mut request = image
            .with_container_name(container_name)
            .with_platform(arena_container::platform::resolve_platform(image_name, image_tag).await)
            .with_mapped_port(port, container_port)
            .with_env_var("ORACLE_PASSWORD", admin_password)
            .with_env_var("APP_USER", database_username)
            .with_env_var("APP_USER_PASSWORD", database_password);

        if database_name != DEFAULT_SERVICE_NAME {
            request = request.with_env_var("ORACLE_DATABASE", database_name);
        }

        if let Some(ref network) = self.network {
            arena_container::network::ensure_network_exists(network).await;
            request = request.with_network(network);
        }

        let container = request.start().await.expect("start oracle container");

        let host = container
            .get_host()
            .await
            .expect("Failed to get host")
            .to_string();
        let host_port = container
            .get_host_port_ipv4(container_port)
            .await
            .expect("Failed to get port");

        {
            let mut meta = self.meta.lock().expect("oracle container meta lock");
            meta.connection_string = Some(format!(
                "//localhost:{DEFAULT_CONTAINER_PORT}/{database_name}"
            ));
            meta.host_address = Some(format!("{host}:{host_port}"));
            meta.database_name = Some(database_name.to_string());
        }
        *container_guard = Some(Arc::new(container));

        tracing::debug!(layer = "oracle_container", phase = "container_started");
    }

    async fn stop(&self) {
        let mut container_guard = self.container.lock().await;
        container_guard.take();
        {
            let mut meta = self.meta.lock().expect("oracle container meta lock");
            meta.connection_string = None;
            meta.host_address = None;
            meta.database_name = None;
        }
        tracing::debug!(layer = "oracle_container", phase = "container_stopped");

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
    }

    fn connection_string(&self) -> Option<String> {
        self.meta
            .lock()
            .expect("oracle container meta lock")
            .connection_string
            .clone()
    }

    fn host_address(&self) -> Option<String> {
        self.meta
            .lock()
            .expect("oracle container meta lock")
            .host_address
            .clone()
    }

    async fn run_sqlplus(
        &self,
        username: &str,
        password: &str,
        script: &str,
    ) -> Result<String, String> {
        let container = {
            let container_guard = self.container.lock().await;
            container_guard
                .clone()
                .ok_or_else(|| "oracle container not started".to_string())?
        };
        let database_name = {
            self.meta
                .lock()
                .expect("oracle container meta lock")
                .database_name
                .clone()
        }
        .ok_or_else(|| "oracle database name not set".to_string())?;

        let connect_target = format!("//localhost:{DEFAULT_CONTAINER_PORT}/{database_name}");
        let cmd = sqlplus::build_exec_command(&connect_target, script);
        let username = username.to_string();
        let password = password.to_string();

        run_exec_on_blocking_pool(container, cmd, username, password).await
    }

    async fn is_container_running(&self) -> bool {
        let container = {
            let container_guard = self.container.lock().await;
            container_guard.clone()
        };

        match container {
            Some(container) => {
                arena_container::container::is_container_running(container.id()).await
            }
            None => false,
        }
    }
}

async fn run_exec_on_blocking_pool(
    container: Arc<testcontainers::core::ContainerAsync<GenericImage>>,
    cmd: Vec<String>,
    username: String,
    password: String,
) -> Result<String, String> {
    let handle = tokio::runtime::Handle::current();

    tokio::task::spawn_blocking(move || {
        handle.block_on(async move {
            let mut result = container
                .exec(ExecCommand::new(cmd).with_env_vars([
                    (sqlplus::SQLPLUS_USER_ENV, username.as_str()),
                    (sqlplus::SQLPLUS_PASS_ENV, password.as_str()),
                ]))
                .await
                .map_err(|e| format!("oracle sqlplus exec failed: {e}"))?;

            let stdout = result
                .stdout_to_vec()
                .await
                .map_err(|e| format!("oracle sqlplus stdout read failed: {e}"))?;
            let stderr = result
                .stderr_to_vec()
                .await
                .map_err(|e| format!("oracle sqlplus stderr read failed: {e}"))?;
            let exit_code = result
                .exit_code()
                .await
                .map_err(|e| format!("oracle sqlplus exit code lookup failed: {e}"))?;

            let stdout = String::from_utf8_lossy(&stdout).into_owned();
            let stderr = String::from_utf8_lossy(&stderr).into_owned();

            if !sqlplus::is_success(exit_code) {
                return Err(format!(
                    "sqlplus exec failed (exit_code={exit_code:?}): stdout={stdout:?} stderr={stderr:?}"
                ));
            }

            Ok(stdout)
        })
    })
    .await
    .unwrap_or_else(|e| Err(format!("oracle sqlplus exec worker panicked: {e}")))
}

pub async fn exec_sql(
    oracle_impl: &dyn OracleImpl,
    username: &str,
    password: &str,
    sql: &str,
) -> Result<String, String> {
    let script = sqlplus::build_script(sql);
    oracle_impl.run_sqlplus(username, password, &script).await
}

pub async fn exec_scalar_query(
    oracle_impl: &dyn OracleImpl,
    username: &str,
    password: &str,
    sql: &str,
) -> Result<i32, String> {
    let stdout = exec_sql(oracle_impl, username, password, sql).await?;
    sqlplus::parse_scalar_i32(&stdout)
}

pub async fn exec_table_list(
    oracle_impl: &dyn OracleImpl,
    username: &str,
    password: &str,
    sql: &str,
) -> Result<Vec<String>, String> {
    let stdout = exec_sql(oracle_impl, username, password, sql).await?;
    Ok(sqlplus::parse_table_list(&stdout))
}

pub async fn exec_constraint_list(
    oracle_impl: &dyn OracleImpl,
    username: &str,
    password: &str,
    sql: &str,
) -> Result<Vec<(String, String)>, String> {
    let stdout = exec_sql(oracle_impl, username, password, sql).await?;
    Ok(sqlplus::parse_constraint_list(&stdout))
}
