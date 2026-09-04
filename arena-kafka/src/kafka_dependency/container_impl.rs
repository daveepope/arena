use crate::kafka_dependency::KafkaImpl;
use async_trait::async_trait;
use std::time::Duration;
use testcontainers_modules::testcontainers::core::{ContainerPort, Healthcheck};
use testcontainers_modules::testcontainers::ImageExt;
use testcontainers_modules::{kafka, testcontainers, testcontainers::runners::AsyncRunner};

pub const KAFKA_INTERNAL_DOCKER_PORT: u16 = 29092;

fn tcp_healthcheck(port: u16) -> Healthcheck {
    Healthcheck::cmd_shell(format!("bash -lc 'echo > /dev/tcp/127.0.0.1/{port}'"))
        .with_interval(Duration::from_millis(250))
        .with_timeout(Duration::from_secs(1))
        .with_retries(40u32)
}

pub(crate) struct KafkaContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<kafka::apache::Kafka>>,
    bootstrap: Option<String>,
    network: Option<String>,
}

impl KafkaContainerImpl {
    pub(crate) fn new(network: Option<String>) -> Self {
        Self {
            container: None,
            bootstrap: None,
            network,
        }
    }
}

#[async_trait]
impl KafkaImpl for KafkaContainerImpl {
    async fn start(&mut self, port: u16, image_name: &str, image_tag: &str, container_name: &str) {
        if self.container.is_some() {
            return;
        }

        arena_container::container::try_remove_existing_container(container_name).await;

        const DEFAULT_CONTAINER_PORT: ContainerPort = kafka::apache::KAFKA_PORT;

        let healthcheck = tcp_healthcheck(DEFAULT_CONTAINER_PORT.as_u16());

        let mut request = kafka::apache::Kafka::default()
            .with_name(image_name)
            .with_tag(image_tag)
            .with_mapped_port(port, DEFAULT_CONTAINER_PORT)
            .with_health_check(healthcheck)
            .with_container_name(container_name)
            .with_platform(arena_container::platform::resolve_platform(image_name, image_tag).await);

        if let Some(ref network) = self.network {
            arena_container::network::ensure_network_exists(network).await;

            // The testcontainers-modules Apache Kafka module's exec_after_start
            // hardcodes KAFKA_ADVERTISED_LISTENERS to only PLAINTEXT + BROKER.
            // Setting it via with_env_var is useless because the start script
            // re-exports it.  We work around this by:
            //   1. Adding DOCKER to KAFKA_LISTENER_SECURITY_PROTOCOL_MAP and
            //      KAFKA_LISTENERS (these env vars ARE respected).
            //   2. Overriding the container cmd so that, after exec_after_start
            //      writes the start script, we `sed` the DOCKER listener into
            //      the advertised-listeners line before execution.
            let start_script = "/opt/kafka/testcontainers_start.sh";

            request = request
                .with_network(network)
                .with_env_var(
                    "KAFKA_LISTENER_SECURITY_PROTOCOL_MAP",
                    "CONTROLLER:PLAINTEXT,PLAINTEXT:PLAINTEXT,BROKER:PLAINTEXT,DOCKER:PLAINTEXT",
                )
                .with_env_var(
                    "KAFKA_LISTENERS",
                    format!(
                        "PLAINTEXT://0.0.0.0:{},BROKER://0.0.0.0:9093,CONTROLLER://0.0.0.0:9094,DOCKER://0.0.0.0:{}",
                        DEFAULT_CONTAINER_PORT.as_u16(),
                        KAFKA_INTERNAL_DOCKER_PORT,
                    ),
                )
                .with_cmd([
                    "-c".to_string(),
                    format!(
                        "while [ ! -f {start_script} ]; do sleep 0.1; done; \
                         sed -i 's|KAFKA_ADVERTISED_LISTENERS=\\(.*\\)|KAFKA_ADVERTISED_LISTENERS=\\1,DOCKER://{container_name}:{KAFKA_INTERNAL_DOCKER_PORT}|' {start_script}; \
                         chmod 755 {start_script} && {start_script}",
                    ),
                ]);
        }

        let container = request.start().await.unwrap_or_else(|e| {
            panic!(
                "{}",
                arena_container::container::start_failure_message("kafka", &e)
            )
        });

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

        tracing::debug!(layer = "kafka_container", phase = "container_started");
    }

    async fn stop(&mut self) {
        self.container.take();
        self.bootstrap = None;
        tracing::debug!(layer = "kafka_container", phase = "container_stopped");

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
    }

    fn bootstrap_servers(&self) -> Option<&str> {
        self.bootstrap.as_deref()
    }
}

pub(crate) struct ConfluentKafkaContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<kafka::confluent::Kafka>>,
    bootstrap: Option<String>,
    network: Option<String>,
}

impl ConfluentKafkaContainerImpl {
    pub(crate) fn new(network: Option<String>) -> Self {
        Self {
            container: None,
            bootstrap: None,
            network,
        }
    }
}

#[async_trait]
impl KafkaImpl for ConfluentKafkaContainerImpl {
    async fn start(&mut self, port: u16, image_name: &str, image_tag: &str, container_name: &str) {
        if self.container.is_some() {
            return;
        }

        // Remove any leftover container with the same name from a previous run
        arena_container::container::try_remove_existing_container(container_name).await;

        const DEFAULT_CONTAINER_PORT: ContainerPort = kafka::confluent::KAFKA_PORT;

        let healthcheck = tcp_healthcheck(DEFAULT_CONTAINER_PORT.as_u16());

        let mut request = kafka::confluent::Kafka::default()
            .with_name(image_name)
            .with_tag(image_tag)
            .with_mapped_port(port, DEFAULT_CONTAINER_PORT)
            .with_health_check(healthcheck)
            .with_container_name(container_name)
            .with_platform(arena_container::platform::resolve_platform(image_name, image_tag).await);

        if let Some(ref network) = self.network {
            arena_container::network::ensure_network_exists(network).await;

            // The Confluent module's exec_after_start dynamically alters
            // advertised.listeners via kafka-configs --alter, overwriting our
            // env-var.  We add DOCKER to the security-protocol-map and
            // listeners env vars (which ARE respected) and override the cmd to
            // re-alter the advertised listeners after the module's own alter
            // completes.
            request = request
                .with_network(network)
                .with_env_var(
                    "KAFKA_LISTENER_SECURITY_PROTOCOL_MAP",
                    "PLAINTEXT:PLAINTEXT,BROKER:PLAINTEXT,DOCKER:PLAINTEXT",
                )
                .with_env_var(
                    "KAFKA_LISTENERS",
                    format!(
                        "PLAINTEXT://0.0.0.0:{},BROKER://0.0.0.0:9092,DOCKER://0.0.0.0:{}",
                        DEFAULT_CONTAINER_PORT.as_u16(),
                        KAFKA_INTERNAL_DOCKER_PORT,
                    ),
                )
                .with_env_var(
                    "KAFKA_ADVERTISED_LISTENERS",
                    format!(
                        "PLAINTEXT://localhost:{port},BROKER://localhost:9092,DOCKER://{container_name}:{KAFKA_INTERNAL_DOCKER_PORT}",
                    ),
                );
        }

        let container = request.start().await.unwrap_or_else(|e| {
            panic!(
                "{}",
                arena_container::container::start_failure_message("kafka", &e)
            )
        });

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

        tracing::debug!(layer = "kafka_container", phase = "container_started");
    }

    async fn stop(&mut self) {
        self.container.take();
        self.bootstrap = None;
        tracing::debug!(layer = "kafka_container", phase = "container_stopped");

        if let Some(ref network) = self.network {
            arena_container::network::remove_network(network).await;
        }
    }

    fn bootstrap_servers(&self) -> Option<&str> {
        self.bootstrap.as_deref()
    }
}
