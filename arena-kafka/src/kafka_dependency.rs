use arena::dependency::RunnableDependency;
use async_trait::async_trait;
use testcontainers_modules::{kafka, testcontainers, testcontainers::runners::AsyncRunner};

#[async_trait]
pub trait KafkaDependencyWrapper: Send + Sync {
    async fn start(&mut self, identifier: &str);
    async fn stop(&mut self, identifier: &str);
}

pub struct KafkaDependency {
    pub identifier: String,
    kafka_wrapper: Box<dyn KafkaDependencyWrapper>,
    dependencies: Vec<Box<dyn RunnableDependency>>,
    running: bool,
}

impl KafkaDependency {
    pub fn new(identifier: String, kafka: Box<dyn KafkaDependencyWrapper>) -> Self {
        KafkaDependency { identifier, kafka_wrapper: kafka, dependencies: vec![], running: false }
    }
}

#[async_trait]
impl RunnableDependency for KafkaDependency {
    async fn start(&mut self) {
        if self.running {
            return;
        }

        log::info!("[Kafka-{}] starting.", self.identifier);

        for dep in self.dependencies.iter_mut() {
            dep.start().await;
        }

        self.kafka_wrapper.start(&self.identifier).await;

        self.running = true;
        log::info!("[Kafka-{}] started.", self.identifier);
    }

    async fn stop(&mut self) {
        if !self.running {
            return;
        }

        log::info!("[Kafka-{}] stopping.", self.identifier);

        self.kafka_wrapper.stop(&self.identifier).await;

        for dep in self.dependencies.iter_mut().rev() {
            dep.stop().await;
        }

        self.running = false;
        log::info!("[Kafka-{}] stopped.", self.identifier);
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.push(dep);
    }
}

pub struct InternalKafkaTestContainerImpl {
    container: Option<testcontainers::core::ContainerAsync<kafka::confluent::Kafka>>,
    bootstrap: Option<String>,
}

impl InternalKafkaTestContainerImpl {
    pub fn new() -> Self {
        Self { container: None, bootstrap: None }
    }

    pub fn bootstrap_servers(&self) -> Option<&str> {
        self.bootstrap.as_deref()
    }
}

#[async_trait]
impl KafkaDependencyWrapper for InternalKafkaTestContainerImpl {
    async fn start(&mut self, identifier: &str) {
        if self.container.is_some() {
            return;
        }

        let container = kafka::confluent::Kafka::default()
            .start()
            .await
            .expect("start kafka container");

        let host = container.get_host().await.expect("Failed to get host").to_string();
        let port = container.get_host_port_ipv4(9092).await.expect("Failed to get port").to_string();

        self.bootstrap = Some(format!("{host}:{port}"));
        self.container = Some(container);

        log::info!("[KafkaImpl-{}] started container.", identifier);
    }

    async fn stop(&mut self, identifier: &str) {
        self.container.take();
        self.bootstrap = None;
        log::info!("[KafkaImpl-{}] stopped container.", identifier);
    }
}