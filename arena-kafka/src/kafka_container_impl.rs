use async_trait::async_trait;
use std::time::Duration;
use testcontainers_modules::{kafka, testcontainers, testcontainers::runners::AsyncRunner};
use testcontainers_modules::testcontainers::core::{ContainerPort, Healthcheck};
use testcontainers_modules::testcontainers::ImageExt;

#[async_trait]
pub trait KafkaImpl: Send + Sync {
    async fn start(&mut self, port: u16, container_tag: &str);
    async fn stop(&mut self);

    fn bootstrap_servers(&self) -> Option<&str>;
}

pub(crate) struct KafkaContainerImpl {
    // `testcontainers-modules` Apache Kafka defaults to `apache/kafka-native`.
    container: Option<testcontainers::core::ContainerAsync<kafka::apache::Kafka>>,
    bootstrap: Option<String>,
}

impl KafkaContainerImpl {
    pub(crate) fn new() -> Self {
        Self {
            container: None,
            bootstrap: None,
        }
    }
}

#[async_trait]
impl KafkaImpl for KafkaContainerImpl {
    async fn start(&mut self, port: u16, container_tag: &str) {
        if self.container.is_some() {
            return;
        }

        const DEFAULT_CONTAINER_PORT: ContainerPort = kafka::apache::KAFKA_PORT;

        // Internal "good enough" healthcheck for now. We'll make it configurable later.
        //
        // This relies on bash's /dev/tcp support. If this ever fails on a future image,
        // we'll swap to a more explicit Kafka readiness command.
        let healthcheck = Healthcheck::cmd_shell(format!(
            "bash -lc 'echo > /dev/tcp/127.0.0.1/{port}'",
            port = DEFAULT_CONTAINER_PORT.as_u16()
        ))
        .with_interval(Duration::from_millis(250))
        .with_timeout(Duration::from_secs(1))
        // 10s / 250ms = 40 attempts (ish)
        .with_retries(40u32);

        let container = kafka::apache::Kafka::default()
            .with_tag(container_tag)
            .with_mapped_port(port, DEFAULT_CONTAINER_PORT)
            .with_health_check(healthcheck)
            .start()
            .await
            .expect("start kafka container");

        let host = container
            .get_host()
            .await
            .expect("Failed to get host")
            .to_string();

        let port = container
            .get_host_port_ipv4(DEFAULT_CONTAINER_PORT)
            .await
            .expect("Failed to get port")
            .to_string();

        self.bootstrap = Some(format!("{host}:{port}"));
        self.container = Some(container);

        log::info!("[KafkaImpl] started container.");
    }

    async fn stop(&mut self) {
        self.container.take();
        self.bootstrap = None;
        log::info!("[KafkaImpl] stopped container.");
    }

    fn bootstrap_servers(&self) -> Option<&str> {
        self.bootstrap.as_deref()
    }
}

pub(crate) struct ConfluentKafkaContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<kafka::confluent::Kafka>>,
    bootstrap: Option<String>,
}

impl ConfluentKafkaContainerImpl {
    pub(crate) fn new() -> Self {
        Self {
            container: None,
            bootstrap: None,
        }
    }
}

#[async_trait]
impl KafkaImpl for ConfluentKafkaContainerImpl {
    async fn start(&mut self, port: u16, container_tag: &str) {
        if self.container.is_some() {
            return;
        }

        const DEFAULT_CONTAINER_PORT: ContainerPort = kafka::confluent::KAFKA_PORT;

        // Internal "good enough" healthcheck for now. We'll make it configurable later.
        let healthcheck = Healthcheck::cmd_shell(format!(
            "bash -lc 'echo > /dev/tcp/127.0.0.1/{port}'",
            port = DEFAULT_CONTAINER_PORT.as_u16()
        ))
        .with_interval(Duration::from_millis(250))
        .with_timeout(Duration::from_secs(1))
        .with_retries(40u32);

        let container = kafka::confluent::Kafka::default()
            .with_tag(container_tag)
            .with_mapped_port(port, DEFAULT_CONTAINER_PORT)
            .with_health_check(healthcheck)
            .start()
            .await
            .expect("start kafka container");

        let host = container
            .get_host()
            .await
            .expect("Failed to get host")
            .to_string();

        let port = container
            .get_host_port_ipv4(DEFAULT_CONTAINER_PORT)
            .await
            .expect("Failed to get port")
            .to_string();

        self.bootstrap = Some(format!("{host}:{port}"));
        self.container = Some(container);

        log::info!("[KafkaImpl] started container.");
    }

    async fn stop(&mut self) {
        self.container.take();
        self.bootstrap = None;
        log::info!("[KafkaImpl] stopped container.");
    }

    fn bootstrap_servers(&self) -> Option<&str> {
        self.bootstrap.as_deref()
    }
}

