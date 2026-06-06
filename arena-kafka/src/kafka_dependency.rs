pub(crate) mod container_impl;
mod healthcheck;
pub(crate) mod topic_creator;

pub use container_impl::KAFKA_INTERNAL_DOCKER_PORT;

use crate::builder::KafkaDependencyBuilder;
use crate::kafka_dependency::healthcheck::DefaultKafkaReadinessCheck;
use crate::kafka_dependency::topic_creator::TopicCreator;
use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use futures_timer::Delay;
use std::time::{Duration, Instant};

#[async_trait]
pub trait KafkaImpl: Send + Sync {
    async fn start(&mut self, port: u16, image_name: &str, image_tag: &str, container_name: &str);
    async fn stop(&mut self);
    fn bootstrap_servers(&self) -> Option<&str>;
}

pub struct KafkaDependency {
    pub identifier: String,
    kafka_impl: Box<dyn KafkaImpl>,
    port: u16,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    running: bool,
    needs_teardown: bool,
    children_started: bool,
    image_name: String,
    image_tag: String,
    container_name: Option<String>,
    readiness_check: Box<dyn ReadinessCheck>,
    topics: Vec<String>,
}

impl KafkaDependency {
    pub(crate) fn new(
        identifier: String,
        kafka_impl: Box<dyn KafkaImpl>,
        port: u16,
        dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
        image_name: String,
        image_tag: String,
        container_name: Option<String>,
        topics: Vec<String>,
    ) -> Self {
        KafkaDependency {
            identifier,
            kafka_impl,
            port,
            dependencies,
            image_name,
            image_tag,
            container_name,
            running: false,
            needs_teardown: false,
            children_started: false,
            readiness_check: Box::new(DefaultKafkaReadinessCheck),
            topics,
        }
    }

    pub fn bootstrap_servers(&self) -> Option<&str> {
        self.kafka_impl.bootstrap_servers()
    }

    pub fn builder(identifier: impl Into<String>) -> KafkaDependencyBuilder {
        KafkaDependencyBuilder::new(identifier)
    }

    pub(crate) fn set_readiness_check(&mut self, check: Box<dyn ReadinessCheck>) {
        self.readiness_check = check;
    }

    fn bootstrap_on_host(&self) -> Result<&str, String> {
        self.bootstrap_servers()
            .ok_or_else(|| "kafka bootstrap servers not available yet".to_string())
    }

    async fn wait_until_ready(&self) {
        let timeout = Duration::from_secs(30);
        let poll_every = Duration::from_millis(100);
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
                    tracing::debug!(
                        dependency = %self.identifier,
                        reason = %err,
                        "kafka bootstrap servers not ready yet"
                    );
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

        match self
            .readiness_check
            .is_ready(&self.identifier, &bootstrap, remaining.as_millis() as u64)
            .await
        {
            Ok(()) => {}
            Err(err) => panic!(
                "[Kafka-{}] readiness check failed: {}",
                self.identifier, err
            ),
        }
    }

    async fn create_topics(&self) {
        if self.topics.is_empty() {
            return;
        }

        let bootstrap = self
            .bootstrap_on_host()
            .expect("bootstrap for topic creation")
            .to_string();

        for topic in &self.topics {
            TopicCreator::create_topic(&bootstrap, topic).unwrap_or_else(|e| {
                panic!(
                    "[Kafka-{}] topic create failed for {topic}: {e}",
                    self.identifier
                )
            });
        }
        tracing::debug!(
            dependency = %self.identifier,
            topic_count = self.topics.len(),
            "topics created"
        );
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

        tracing::debug!(dependency = %self.identifier, phase = "start_begin", "starting");
        let sw = Instant::now();

        if let Some(children) = self.dependencies.as_mut() {
            if !children.is_empty() {
                self.children_started = true;
                for dep in children.iter_mut() {
                    dep.start().await;
                }
            }
        }

        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = arena_container::identifier::resolve_container_name(
            &self.identifier,
            self.container_name.as_deref(),
        );

        let sw_container = Instant::now();
        self.needs_teardown = true;
        self.kafka_impl
            .start(self.port, &image_name, &image_tag, &container_name)
            .await;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_container.elapsed(),
            "container start finished"
        );

        let sw_ready = Instant::now();
        self.wait_until_ready().await;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_ready.elapsed(),
            "readiness wait finished"
        );

        self.create_topics().await;

        self.running = true;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "started"
        );
    }

    async fn stop(&mut self) {
        self.kafka_impl.stop().await;
        self.needs_teardown = false;

        if !self.running {
            if self.children_started {
                for dep in self.dependencies.iter_mut().flatten().rev() {
                    dep.stop().await;
                }
                self.children_started = false;
            }
            return;
        }

        tracing::debug!(dependency = %self.identifier, phase = "stop_begin", "stopping");
        let sw = Instant::now();

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.stop().await;
        }

        self.children_started = false;
        self.running = false;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "stopped"
        );
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.get_or_insert_with(Vec::new).push(dep);
    }

    async fn soft_reset(&self) {
        if !self.running {
            return;
        }

        let bootstrap = self
            .bootstrap_on_host()
            .expect("bootstrap for soft reset")
            .to_string();

        for topic in &self.topics {
            if let Err(e) = TopicCreator::clear_messages(&bootstrap, topic) {
                tracing::warn!(
                    dependency = %self.identifier,
                    topic = %topic,
                    error = %e,
                    "soft reset: topic clear failed"
                );
            } else {
                tracing::debug!(
                    dependency = %self.identifier,
                    topic = %topic,
                    "soft reset: topic messages cleared"
                );
            }
        }
    }

    async fn hard_reset(&mut self) {
        if !self.running {
            return;
        }

        tracing::debug!(
            dependency = %self.identifier,
            phase = "hard_reset",
            "restarting kafka container"
        );
        let image_name = self.image_name.clone();
        let image_tag = self.image_tag.clone();
        let container_name = arena_container::identifier::resolve_container_name(
            &self.identifier,
            self.container_name.as_deref(),
        );

        self.kafka_impl.stop().await;
        self.running = false;

        self.kafka_impl
            .start(self.port, &image_name, &image_tag, &container_name)
            .await;
        self.wait_until_ready().await;
        self.create_topics().await;
        self.running = true;
    }
}

impl Drop for KafkaDependency {
    fn drop(&mut self) {
        if self.running {
            tracing::warn!(
                dependency = %self.identifier,
                "drop while running; forcing stop"
            );
            futures::executor::block_on(<Self as RunnableDependency>::stop(self));
        } else if self.needs_teardown || self.children_started {
            futures::executor::block_on(<Self as RunnableDependency>::stop(self));
        }
    }
}
