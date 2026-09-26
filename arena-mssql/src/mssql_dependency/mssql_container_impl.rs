use async_trait::async_trait;
use std::time::Duration;
use testcontainers_modules::testcontainers::core::ContainerPort;
use testcontainers_modules::testcontainers::ImageExt;
use testcontainers_modules::{mssql_server, testcontainers, testcontainers::runners::AsyncRunner};
use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const CONNECT_RETRY_ATTEMPTS: u32 = 3;
const CONNECT_RETRY_BACKOFF_BASE: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MssqlEncryption {
    Off,
    On,
}

impl Default for MssqlEncryption {
    fn default() -> Self {
        MssqlEncryption::Off
    }
}

#[async_trait]
pub trait MssqlImpl: Send + Sync {
    fn set_expiry(&mut self, _expiry: Option<Duration>) {}
    async fn start(
        &mut self,
        port: u16,
        database_name: &str,
        database_username: &str,
        database_password: &str,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    ) -> Result<(), String>;
    async fn stop(&mut self) -> Result<(), String>;
    async fn force_stop(&mut self) -> bool;
    fn release(&mut self);

    fn connection_string(&self) -> Option<&str>;

    fn admin_connection_string(&self) -> Option<&str>;
}

pub(crate) struct MssqlContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<mssql_server::MssqlServer>>,
    connection_string: Option<String>,
    admin_connection_string: Option<String>,
    network: Option<String>,
    encryption: MssqlEncryption,
    container_name: Option<String>,
    expiry: Option<Duration>,
}

impl MssqlContainerImpl {
    pub(crate) fn new(network: Option<String>, encryption: MssqlEncryption) -> Self {
        Self {
            container: None,
            connection_string: None,
            admin_connection_string: None,
            network,
            encryption,
            container_name: None,
            expiry: Some(arena_container::expiry::DEFAULT_EXPIRY),
        }
    }
}

#[async_trait]
impl MssqlImpl for MssqlContainerImpl {
    fn set_expiry(&mut self, expiry: Option<Duration>) {
        self.expiry = expiry;
    }

    async fn start(
        &mut self,
        port: u16,
        database_name: &str,
        database_username: &str,
        database_password: &str,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    ) -> Result<(), String> {
        if self.container.is_some() {
            return Ok(());
        }

        arena_container::expiry::remove_expired_containers_if_enabled(crate::MODULE, self.expiry)
            .await;

        arena_container::container::try_remove_existing_container(container_name).await;

        const DEFAULT_CONTAINER_PORT: u16 = 1433;

        let image = mssql_server::MssqlServer::default()
            .with_accept_eula()
            .with_sa_password(database_password);

        let mut request = image
            .with_mapped_port(port, ContainerPort::from(DEFAULT_CONTAINER_PORT))
            .with_name(image_name)
            .with_tag(image_tag)
            .with_container_name(container_name)
            .with_labels(arena_container::expiry::expiry_labels_for(
                crate::MODULE,
                self.expiry,
            ))
            .with_platform(arena_container::platform::resolve_platform(image_name, image_tag).await);

        if let Some(ref network) = self.network {
            arena_container::network::ensure_network_exists(network).await;
            request = request.with_network(network);
        }

        let container = request
            .start()
            .await
            .map_err(|e| arena_container::container::start_failure_message("mssql", &e))?;

        let host = container
            .get_host()
            .await
            .map_err(|e| format!("mssql container host unavailable: {e}"))?
            .to_string();

        let host_port = container
            .get_host_port_ipv4(DEFAULT_CONTAINER_PORT)
            .await
            .map_err(|e| format!("mssql port unavailable: {e}"))?;

        self.admin_connection_string = Some(build_ado_connection_string(
            &host,
            host_port,
            "master",
            database_username,
            database_password,
            self.encryption,
        ));
        self.connection_string = Some(build_ado_connection_string(
            &host,
            host_port,
            database_name,
            database_username,
            database_password,
            self.encryption,
        ));
        self.container = Some(container);
        self.container_name = Some(container_name.to_string());

        tracing::debug!(layer = "mssql_container", phase = "container_started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.container.take();
        self.connection_string = None;
        self.admin_connection_string = None;
        tracing::debug!(layer = "mssql_container", phase = "container_stopped");

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
        Ok(())
    }

    fn release(&mut self) {
        self.container.take();
        self.connection_string = None;
        self.admin_connection_string = None;
    }

    async fn force_stop(&mut self) -> bool {
        self.release();

        let removed = match self.container_name.as_deref() {
            Some(name) => arena_container::container::force_remove_container(name).await,
            None => true,
        };

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
        removed
    }

    fn connection_string(&self) -> Option<&str> {
        self.connection_string.as_deref()
    }

    fn admin_connection_string(&self) -> Option<&str> {
        self.admin_connection_string.as_deref()
    }
}

pub fn build_ado_connection_string(
    host: &str,
    port: u16,
    database_name: &str,
    username: &str,
    password: &str,
    encryption: MssqlEncryption,
) -> String {
    let base = format!(
        "Server=tcp:{host},{port};Database={database_name};User Id={username};Password={password};TrustServerCertificate=True;"
    );
    match encryption {
        MssqlEncryption::Off => format!("{base}encrypt=DANGER_PLAINTEXT;"),
        MssqlEncryption::On => base,
    }
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

    let Some(budget) = timeout else {
        return connect_with_config(config).await;
    };

    connect_with_retry(config, budget, CONNECT_RETRY_ATTEMPTS).await
}

async fn connect_with_retry(
    config: Config,
    budget: Duration,
    attempts: u32,
) -> Result<Client<Compat<TcpStream>>, String> {
    let mut last_err = String::new();

    for attempt in 0..attempts {
        let outcome = tokio::time::timeout(budget, connect_with_config(config.clone())).await;
        last_err = match outcome {
            Ok(Ok(client)) => return Ok(client),
            Ok(Err(err)) => err,
            Err(_) => format!("mssql connect exceeded {budget:?}"),
        };

        if attempt + 1 < attempts {
            tokio::time::sleep(CONNECT_RETRY_BACKOFF_BASE * 2u32.pow(attempt)).await;
        }
    }

    Err(format!(
        "mssql connect failed after {attempts} attempts: {last_err}"
    ))
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

