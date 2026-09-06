use async_trait::async_trait;
use std::time::Duration;
use testcontainers_modules::testcontainers::core::{ContainerPort, Healthcheck};
use testcontainers_modules::testcontainers::ImageExt;
use testcontainers_modules::{postgres, testcontainers, testcontainers::runners::AsyncRunner};

#[async_trait]
pub trait PostgresImpl: Send + Sync {
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
    fn set_expiry(&mut self, _expiry: Option<Duration>) {}

    fn connection_string(&self) -> Option<&str>;
}

pub(crate) struct PostgresContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<postgres::Postgres>>,
    connection_string: Option<String>,
    network: Option<String>,
    container_name: Option<String>,
    expiry: Option<Duration>,
}

impl PostgresContainerImpl {
    pub(crate) fn new(network: Option<String>) -> Self {
        Self {
            container: None,
            connection_string: None,
            network,
            container_name: None,
            expiry: Some(arena_container::expiry::DEFAULT_EXPIRY),
        }
    }
}

#[async_trait]
impl PostgresImpl for PostgresContainerImpl {
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

        const DEFAULT_CONTAINER_PORT: u16 = 5432;

        let healthcheck = Healthcheck::cmd_shell(format!(
            "pg_isready -h 127.0.0.1 -p {DEFAULT_CONTAINER_PORT} -U {database_username} -d {database_name}"
        ))
        .with_interval(Duration::from_millis(250))
        .with_timeout(Duration::from_secs(1))
        .with_retries(40u32);

        let image = postgres::Postgres::default()
            .with_db_name(database_name)
            .with_user(database_username)
            .with_password(database_password);

        let mut request = image
            .with_mapped_port(port, ContainerPort::from(DEFAULT_CONTAINER_PORT))
            .with_health_check(healthcheck)
            .with_name(image_name)
            .with_tag(image_tag)
            .with_container_name(container_name)
            .with_platform(arena_container::platform::resolve_platform(image_name, image_tag).await);

        request = request.with_labels(arena_container::expiry::expiry_labels_for(
            crate::MODULE,
            self.expiry,
        ));

        if let Some(ref network) = self.network {
            arena_container::network::ensure_network_exists(network).await;
            request = request.with_network(network);
        }

        let container = request
            .start()
            .await
            .map_err(|e| arena_container::container::start_failure_message("postgres", &e))?;

        let host = container
            .get_host()
            .await
            .map_err(|e| format!("postgres container host unavailable: {e}"))?
            .to_string();

        let port = container
            .get_host_port_ipv4(DEFAULT_CONTAINER_PORT)
            .await
            .map_err(|e| format!("postgres port unavailable: {e}"))?
            .to_string();
        self.connection_string = Some(format!(
            "postgres://{database_username}:{database_password}@{host}:{port}/{database_name}"
        ));
        self.container = Some(container);
        self.container_name = Some(container_name.to_string());

        tracing::debug!(layer = "postgres_container", phase = "container_started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.container.take();
        self.connection_string = None;
        tracing::debug!(layer = "postgres_container", phase = "container_stopped");

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
        Ok(())
    }

    fn release(&mut self) {
        self.container.take();
        self.connection_string = None;
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
}
