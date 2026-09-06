pub mod client;
pub(crate) mod container_impl;
mod healthcheck;
pub(crate) mod topic_creator;

pub use container_impl::KAFKA_INTERNAL_DOCKER_PORT;

use crate::builder::KafkaDependencyBuilder;
use crate::kafka_dependency::healthcheck::DefaultKafkaReadinessCheck;
use crate::kafka_dependency::topic_creator::TopicCreator;
use arena::dependency::{Dependency, RunnableDependency};
use arena::lifecycle::{Fault, RunnableState};
use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use futures_timer::Delay;
use std::time::{Duration, Instant};

#[async_trait]
pub trait KafkaImpl: Send + Sync {
    fn set_expiry(&mut self, _expiry: Option<Duration>) {}
    async fn start(
        &mut self,
        port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    ) -> Result<(), String>;
    async fn stop(&mut self) -> Result<(), String>;
    async fn force_stop(&mut self) -> bool;
    fn release(&mut self);
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
    state: RunnableState,
    faults: Vec<Fault>,
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
            state: RunnableState::NotStarted,
            faults: Vec::new(),
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

    async fn wait_until_ready(&self) -> Result<(), String> {
        let timeout = Duration::from_secs(30);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();

        let bootstrap = loop {
            if start.elapsed() >= timeout {
                return Err(format!("kafka did not become ready within {timeout:?}"));
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
            return Err(format!("kafka did not become ready within {timeout:?}"));
        }

        self.readiness_check
            .is_ready(&self.identifier, &bootstrap, remaining.as_millis() as u64)
            .await
            .map_err(|err| format!("readiness check failed: {err}"))
    }

    async fn fail(&mut self, message: impl Into<String>, causes: Vec<Fault>) -> Fault {
        let fault = Fault::dependency(&self.identifier, message).caused_by_all(causes);
        self.faults.push(fault.clone());
        <Self as RunnableDependency>::force_stop(self).await;
        fault
    }

    async fn create_topics(&self) -> Result<(), String> {
        if self.topics.is_empty() {
            return Ok(());
        }

        let bootstrap = self.bootstrap_on_host()?.to_string();

        let kafka_client = client::connect_client(&bootstrap)
            .await
            .map_err(|e| format!("connect failed: {e}"))?;

        for topic in &self.topics {
            TopicCreator::create_topic(&kafka_client, topic)
                .await
                .map_err(|e| format!("topic create failed for {topic}: {e}"))?;
        }
        tracing::debug!(
            dependency = %self.identifier,
            topic_count = self.topics.len(),
            "topics created"
        );
        Ok(())
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

    fn state(&self) -> RunnableState {
        self.state
    }

    fn faults(&self) -> &[Fault] {
        &self.faults
    }

    async fn start(&mut self) -> Result<(), Fault> {
        if self.running {
            return Ok(());
        }
        self.state = RunnableState::Starting;

        tracing::debug!(dependency = %self.identifier, phase = "start_begin", "starting");
        let sw = Instant::now();

        if let Some(children) = self.dependencies.as_mut() {
            if !children.is_empty() {
                self.children_started = true;
                let mut child_faults = Vec::new();
                for dep in children.iter_mut() {
                    if let Err(fault) = dep.start().await {
                        child_faults.push(fault);
                    }
                }
                if !child_faults.is_empty() {
                    return Err(self.fail("child dependency failed to start", child_faults).await);
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
        if let Err(message) = self
            .kafka_impl
            .start(self.port, &image_name, &image_tag, &container_name)
            .await
        {
            return Err(self.fail(message, Vec::new()).await);
        }
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_container.elapsed(),
            "container start finished"
        );

        let sw_ready = Instant::now();
        self.state = RunnableState::ReadinessCheck;
        if let Err(message) = self.wait_until_ready().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw_ready.elapsed(),
            "readiness wait finished"
        );

        if let Err(message) = self.create_topics().await {
            return Err(self.fail(message, Vec::new()).await);
        }

        self.running = true;
        self.state = RunnableState::Started;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "started"
        );
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        self.state = RunnableState::Stopping;
        let mut causes = Vec::new();

        if let Err(message) = self.kafka_impl.stop().await {
            causes.push(Fault::dependency(&self.identifier, message));
        }
        self.needs_teardown = false;

        tracing::debug!(dependency = %self.identifier, phase = "stop_begin", "stopping");
        let sw = Instant::now();

        for dep in self.dependencies.iter_mut().flatten().rev() {
            if let Err(fault) = dep.stop().await {
                causes.push(fault);
            }
        }

        self.children_started = false;
        self.running = false;

        if !causes.is_empty() {
            let fault =
                Fault::dependency(&self.identifier, "stop did not complete").caused_by_all(causes);
            self.faults.push(fault.clone());
            self.state = RunnableState::Faulted;
            return Err(fault);
        }

        self.state = RunnableState::Stopped;
        tracing::debug!(
            dependency = %self.identifier,
            elapsed = ?sw.elapsed(),
            "stopped"
        );
        Ok(())
    }

    fn release(&mut self) {
        self.kafka_impl.release();
        self.running = false;
        self.needs_teardown = false;
        self.children_started = false;
        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.release();
        }
        self.state = RunnableState::Stopped;
    }

    async fn force_stop(&mut self) {
        let removed = self.kafka_impl.force_stop().await;
        self.needs_teardown = false;
        self.running = false;
        self.children_started = false;

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.force_stop().await;
        }

        if removed {
            self.state = RunnableState::Stopped;
            return;
        }

        let unconfirmed = Fault::dependency(
            &self.identifier,
            "forced teardown could not confirm the container was removed",
        );
        if !self
            .faults
            .iter()
            .any(|f| f.message == unconfirmed.message && f.id == unconfirmed.id)
        {
            self.faults.push(unconfirmed);
        }
        self.state = RunnableState::Faulted;
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.get_or_insert_with(Vec::new).push(dep);
    }

    fn children(&self) -> &[Dependency] {
        self.dependencies.as_deref().unwrap_or(&[])
    }

    fn children_mut(&mut self) -> &mut [Dependency] {
        self.dependencies.as_deref_mut().unwrap_or(&mut [])
    }

    async fn soft_reset(&self) -> Result<(), Fault> {
        if !self.running {
            return Ok(());
        }

        let bootstrap = self
            .bootstrap_on_host()
            .map_err(|message| Fault::dependency(&self.identifier, message))?
            .to_string();

        let kafka_client = match client::connect_client(&bootstrap).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    dependency = %self.identifier,
                    error = %e,
                    "soft reset: kafka connect failed"
                );
                return Ok(());
            }
        };

        for topic in &self.topics {
            if let Err(e) = TopicCreator::clear_messages(&kafka_client, topic).await {
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
        Ok(())
    }

    async fn hard_reset(&mut self) -> Result<(), Fault> {
        if !self.running {
            return Ok(());
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

        if let Err(message) = self.kafka_impl.stop().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        self.running = false;

        if let Err(message) = self
            .kafka_impl
            .start(self.port, &image_name, &image_tag, &container_name)
            .await
        {
            return Err(self.fail(message, Vec::new()).await);
        }
        if let Err(message) = self.wait_until_ready().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        if let Err(message) = self.create_topics().await {
            return Err(self.fail(message, Vec::new()).await);
        }
        self.running = true;
        self.state = RunnableState::Started;
        Ok(())
    }
}

impl Drop for KafkaDependency {
    fn drop(&mut self) {
        if self.running || self.needs_teardown || self.children_started {
            tracing::warn!(
                dependency = %self.identifier,
                "drop while running; releasing container"
            );
            self.kafka_impl.release();
            self.running = false;
            self.needs_teardown = false;
            self.children_started = false;
            self.state = RunnableState::Stopped;
        }
    }
}
