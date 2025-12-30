use arena::dependency::RunnableDependency;
use async_trait::async_trait;
use testcontainers_modules::{kafka, testcontainers, testcontainers::runners::AsyncRunner};

#[async_trait]
pub trait KafkaImpl: Send + Sync {
    async fn start(&mut self, identifier: &str);
    async fn stop(&mut self, identifier: &str);
}

pub struct DockerKafkaImpl {
    container: Option<testcontainers::core::Container<kafka::confluent::Kafka>>,
    bootstrap: Option<String>,
}

impl DockerKafkaImpl {
    pub fn new() -> Self {
        Self { container: None, bootstrap: None }
    }

    pub fn bootstrap_servers(&self) -> Option<&str> {
        self.bootstrap.as_deref()
    }
}

#[async_trait]
impl KafkaImpl for DockerKafkaImpl {
    async fn start(&mut self, identifier: &str) {
        if self.container.is_some() {
            return;
        }

        let container = kafka::confluent::Kafka::default()
            .start()
            .await
            .expect("start kafka container");

        // NOTE: depending on versions, these may or may not be async. If the compiler
        // says “not a future”, just remove `.await`.
        let host = container.get_host().to_string();
        let port = container.get_host_port_ipv4(9092).expect("mapped kafka port");

        self.bootstrap = Some(format!("{host}:{port}"));
        self.container = Some(container);

        println!("[KafkaImpl-{}] started container.", identifier);
    }

    async fn stop(&mut self, identifier: &str) {
        self.container.take(); // drop == stop container
        self.bootstrap = None;
        println!("[KafkaImpl-{}] stopped container.", identifier);
    }
}

pub struct KafkaDependency {
    pub identifier: String,
    kafka: Box<dyn KafkaImpl>,
    dependencies: Vec<Box<dyn RunnableDependency>>,
    running: bool,
}

impl KafkaDependency {
    pub fn new(identifier: String, kafka: Box<dyn KafkaImpl>) -> Self {
        KafkaDependency { identifier, kafka, dependencies: vec![], running: false }
    }
}

#[async_trait]
impl RunnableDependency for KafkaDependency {
    async fn start(&mut self) {
        if self.running {
            return;
        }

        println!("[Kafka-{}] starting.", self.identifier);

        for dep in self.dependencies.iter_mut() {
            dep.start().await;
        }

        self.kafka.start(&self.identifier).await;

        self.running = true;
        println!("[Kafka-{}] started.", self.identifier);
    }

    async fn stop(&mut self) {
        if !self.running {
            return;
        }

        println!("[Kafka-{}] stopping.", self.identifier);

        self.kafka.stop(&self.identifier).await;

        for dep in self.dependencies.iter_mut().rev() {
            dep.stop().await;
        }

        self.running = false;
        println!("[Kafka-{}] stopped.", self.identifier);
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.push(dep);
    }
}