#[cfg(test)]
mod unit_tests;
mod healthcheck;
pub(crate) mod container_impl;

use arena::dependency::RunnableDependency;
use async_trait::async_trait;
use crate::builder::KafkaDependencyBuilder;
use futures_timer::Delay;
use crate::kafka_dependency::healthcheck::{KafkaHealthcheckOps, RdkafkaHealthcheckOps};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[async_trait]
pub trait KafkaImpl: Send + Sync {
    async fn start(&mut self, port: u16, image_tag: &str, container_name: &str);
    async fn stop(&mut self);
    fn bootstrap_servers(&self) -> Option<&str>;
}

fn sanitize_identifier(input: &str) -> String {
    let mut safe = String::with_capacity(input.len());
    for c in input.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            safe.push(c);
        } else {
            safe.push('-');
        }
    }
    safe
}

pub struct KafkaDependency {
    pub identifier: String,
    kafka_impl: Box<dyn KafkaImpl>,
    port: u16,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    running: bool,
    image_tag: String,
    container_name: Option<String>,
    healthcheck_ops: Box<dyn KafkaHealthcheckOps>,
}

impl KafkaDependency {
    pub fn new(
        identifier: String,
        kafka_impl: Box<dyn KafkaImpl>,
        port: u16,
        dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
        image_tag: String,
        container_name: Option<String>,
    ) -> Self {
        KafkaDependency {
            identifier,
            kafka_impl,
            port,
            dependencies,
            image_tag,
            container_name,
            running: false,
            healthcheck_ops: Box::new(RdkafkaHealthcheckOps),
        }
    }

    pub fn bootstrap_servers(&self) -> Option<&str> {
        self.kafka_impl.bootstrap_servers()
    }

    pub fn builder(identifier: impl Into<String>) -> KafkaDependencyBuilder {
        KafkaDependencyBuilder::new(identifier)
    }

    fn default_container_name(&self) -> String {
        let safe = sanitize_identifier(&self.identifier);

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        format!("arena-kafka-{safe}-{ts}")
    }

    fn bootstrap_on_host(&self) -> Result<&str, String> {
        self.bootstrap_servers()
            .ok_or_else(|| "kafka bootstrap servers not available yet".to_string())
    }

    async fn wait_until_ready(&self) {
        let timeout = Duration::from_secs(15);
        let poll_every = Duration::from_millis(250);
        let start = Instant::now();

        let bootstrap = loop {
            if start.elapsed() >= timeout {
                panic!(
                    "[Kafka-{}] kafka did not become ready within {:?}",
                    self.identifier, timeout
                );
            }

            match self.bootstrap_on_host() {
                Ok(v) => break v.to_string(),
                Err(err) => {
                    log::debug!("[Kafka-{}] readiness bootstrap missing: {}", self.identifier, err);
                    Delay::new(poll_every).await;
                }
            }
        };

        let remaining = timeout.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            panic!(
                "[Kafka-{}] kafka did not become ready within {:?}",
                self.identifier, timeout
            );
        }

        match healthcheck::run_with_retry(
            self.healthcheck_ops.as_ref(),
            &self.identifier,
            &bootstrap,
            remaining,
        )
        .await
        {
            Ok(()) => {}
            Err(err) => panic!("[Kafka-{}] readiness check failed: {}", self.identifier, err),
        };
    }
}

#[async_trait]
impl RunnableDependency for KafkaDependency {
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

        log::info!("[Kafka-{}] starting.", self.identifier);
        let sw = Instant::now();

        for dep in self.dependencies.iter_mut().flatten() {
            dep.start().await;
        }

        let image_tag = self.image_tag.clone();
        let container_name = self
            .container_name
            .clone()
            .unwrap_or_else(|| self.default_container_name());

        let sw_container = Instant::now();
        self.kafka_impl
            .start(self.port, &image_tag, &container_name)
            .await;
        log::debug!(
            "[Kafka-{}] container start in {:?}.",
            self.identifier,
            sw_container.elapsed()
        );

        let sw_ready = Instant::now();
        self.wait_until_ready().await;
        log::debug!(
            "[Kafka-{}] readiness in {:?}.",
            self.identifier,
            sw_ready.elapsed()
        );

        self.running = true;
        log::debug!(
            "[Kafka-{}] start complete in {:?}.",
            self.identifier,
            sw.elapsed()
        );
        log::info!("[Kafka-{}] started.", self.identifier);
    }

    async fn stop(&mut self) {
        if !self.running {
            return;
        }

        log::info!("[Kafka-{}] stopping.", self.identifier);
        let sw = Instant::now();

        self.kafka_impl.stop().await;

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.stop().await;
        }

        self.running = false;
        log::debug!(
            "[Kafka-{}] stop complete in {:?}.",
            self.identifier,
            sw.elapsed()
        );
        log::info!("[Kafka-{}] stopped.", self.identifier);
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.get_or_insert_with(Vec::new).push(dep);
    }
}