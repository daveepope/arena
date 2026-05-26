use async_trait::async_trait;
use std::time::Duration;
use testcontainers_modules::testcontainers::core::ContainerPort;
use testcontainers_modules::testcontainers::ImageExt;
use testcontainers_modules::{mssql_server, testcontainers, testcontainers::runners::AsyncRunner};
use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

#[async_trait]
pub trait MssqlImpl: Send + Sync {
    async fn start(
        &mut self,
        port: u16,
        database_name: &str,
        database_username: &str,
        database_password: &str,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    );
    async fn stop(&mut self);

    fn connection_string(&self) -> Option<&str>;

    fn admin_connection_string(&self) -> Option<&str>;
}

pub(crate) struct MssqlContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<mssql_server::MssqlServer>>,
    connection_string: Option<String>,
    admin_connection_string: Option<String>,
    network: Option<String>,
}

impl MssqlContainerImpl {
    pub(crate) fn new(network: Option<String>) -> Self {
        Self {
            container: None,
            connection_string: None,
            admin_connection_string: None,
            network,
        }
    }
}

#[async_trait]
impl MssqlImpl for MssqlContainerImpl {
    async fn start(
        &mut self,
        port: u16,
        database_name: &str,
        database_username: &str,
        database_password: &str,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    ) {
        if self.container.is_some() {
            return;
        }

        arena_container::container::try_remove_existing_container(container_name).await;

        const DEFAULT_CONTAINER_PORT: u16 = 1433;

        let image = mssql_server::MssqlServer::default()
            .with_accept_eula()
            .with_sa_password(database_password);

        let mut request = image
            .with_mapped_port(port, ContainerPort::from(DEFAULT_CONTAINER_PORT))
            .with_name(image_name)
            .with_tag(image_tag)
            .with_container_name(container_name);

        if let Some(ref network) = self.network {
            arena_container::network::ensure_network_exists(network).await;
            request = request.with_network(network);
        }

        let container = request.start().await.expect("start mssql container");

        let host = container
            .get_host()
            .await
            .expect("Failed to get host")
            .to_string();

        let host_port = container
            .get_host_port_ipv4(DEFAULT_CONTAINER_PORT)
            .await
            .expect("Failed to get port");

        self.admin_connection_string = Some(build_ado_connection_string(
            &host,
            host_port,
            "master",
            database_username,
            database_password,
        ));
        self.connection_string = Some(build_ado_connection_string(
            &host,
            host_port,
            database_name,
            database_username,
            database_password,
        ));
        self.container = Some(container);

        tracing::debug!(layer = "mssql_container", phase = "container_started");
    }

    async fn stop(&mut self) {
        self.container.take();
        self.connection_string = None;
        self.admin_connection_string = None;
        tracing::debug!(layer = "mssql_container", phase = "container_stopped");

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
    }

    fn connection_string(&self) -> Option<&str> {
        self.connection_string.as_deref()
    }

    fn admin_connection_string(&self) -> Option<&str> {
        self.admin_connection_string.as_deref()
    }
}

pub(crate) fn build_ado_connection_string(
    host: &str,
    port: u16,
    database_name: &str,
    username: &str,
    password: &str,
) -> String {
    format!(
        "Server=tcp:{host},{port};Database={database_name};User Id={username};Password={password};TrustServerCertificate=True;"
    )
}

pub async fn connect(connection_string: &str) -> Result<Client<Compat<TcpStream>>, String> {
    connect_with_timeout(connection_string, Some(DEFAULT_CONNECT_TIMEOUT)).await
}

pub async fn connect_with_timeout(
    connection_string: &str,
    timeout: Option<Duration>,
) -> Result<Client<Compat<TcpStream>>, String> {
    let config = Config::from_ado_string(connection_string)
        .map_err(|e| format!("parse ADO connection string: {e}"))?;

    let fut = connect_with_config(config);
    match timeout {
        Some(budget) => tokio::time::timeout(budget, fut)
            .await
            .map_err(|_| format!("mssql connect exceeded {budget:?}"))?,
        None => fut.await,
    }
}

pub(crate) async fn connect_with_config(
    mut config: Config,
) -> Result<Client<Compat<TcpStream>>, String> {
    config.trust_cert();

    let tcp = TcpStream::connect(config.get_addr())
        .await
        .map_err(|e| format!("tcp connect failed: {e}"))?;
    tcp.set_nodelay(true)
        .map_err(|e| format!("set_nodelay failed: {e}"))?;

    Client::connect(config, tcp.compat_write())
        .await
        .map_err(|e| format!("tiberius connect failed: {e}"))
}

#[allow(dead_code)]
pub(crate) fn config_from_parts(
    host: &str,
    port: u16,
    database_name: &str,
    username: &str,
    password: &str,
) -> Config {
    let mut config = Config::new();
    config.host(host);
    config.port(port);
    config.database(database_name);
    config.authentication(AuthMethod::sql_server(username, password));
    config.trust_cert();
    config
}
