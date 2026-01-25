mod tests;

use arena::dependency::RunnableDependency;
use async_trait::async_trait;
use crate::builder::KafkaDependencyBuilder;
use crate::kafka_container_impl::KafkaImpl;
use futures_timer::Delay;
use rdkafka::config::ClientConfig;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[async_trait]
trait KafkaHealthcheckOps: Send + Sync {
    async fn create_topic(&self, bootstrap: &str, topic: &str) -> Result<(), String>;
    async fn delete_topic(&self, bootstrap: &str, topic: &str) -> Result<(), String>;
    async fn publish(&self, bootstrap: &str, topic: &str, payload: &str) -> Result<(), String>;
    async fn consume_verify(
        &self,
        bootstrap: &str,
        topic: &str,
        expected_payload: &str,
    ) -> Result<bool, String>;
}

struct RdkafkaHealthcheckOps;

#[async_trait]
impl KafkaHealthcheckOps for RdkafkaHealthcheckOps {
    async fn create_topic(&self, bootstrap: &str, topic: &str) -> Result<(), String> {
        let admin: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .create()
            .map_err(|e| format!("create kafka admin client failed: {e}"))?;

        let new_topic = NewTopic::new(topic, 1, TopicReplication::Fixed(1));
        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(2)));

        match admin.create_topics([&new_topic], &opts).await {
            Ok(results) => {
                for r in results {
                    if let Err((_t, e)) = r {
                        if e.to_string().to_lowercase().contains("already exists") {
                            return Ok(());
                        }
                        return Err(format!("kafka topic create failed: {e}"));
                    }
                }
                Ok(())
            }
            Err(err) => Err(format!("kafka topic create request failed: {err}")),
        }
    }

    async fn delete_topic(&self, bootstrap: &str, topic: &str) -> Result<(), String> {
        let admin: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .create()
            .map_err(|e| format!("create kafka admin client failed: {e}"))?;

        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(2)));
        admin
            .delete_topics(&[topic], &opts)
            .await
            .map(|_res| ())
            .map_err(|e| format!("kafka topic delete failed: {e}"))
    }

    async fn publish(&self, bootstrap: &str, topic: &str, payload: &str) -> Result<(), String> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .set("message.timeout.ms", "2000")
            .create()
            .map_err(|e| format!("create kafka producer failed: {e}"))?;

        let record = FutureRecord::to(topic).key("healthcheck").payload(payload);
        producer
            .send(record, Duration::from_secs(2))
            .await
            .map(|_| ())
            .map_err(|(e, _msg)| format!("kafka publish failed: {e}"))
    }

    async fn consume_verify(
        &self,
        bootstrap: &str,
        topic: &str,
        expected_payload: &str,
    ) -> Result<bool, String> {
        let consumer: BaseConsumer = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .set("group.id", format!("arena-healthcheck-{topic}"))
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .set("session.timeout.ms", "6000")
            .create()
            .map_err(|e| format!("create kafka consumer failed: {e}"))?;

        consumer
            .subscribe(&[topic])
            .map_err(|e| format!("kafka subscribe failed: {e}"))?;

        let consume_deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < consume_deadline {
            match consumer.poll(Duration::from_millis(250)) {
                None => {}
                Some(Err(err)) => return Err(format!("kafka consume failed: {err}")),
                Some(Ok(msg)) => {
                    if let Some(bytes) = msg.payload() {
                        if bytes == expected_payload.as_bytes() {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        Ok(false)
    }
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

    fn default_container_name(&self) -> String {
        let safe = Self::sanitize_identifier(&self.identifier);

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

    fn healthcheck_topic_name(identifier: &str) -> String {
        let safe = Self::sanitize_identifier(identifier);

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        format!("arena_healthcheck_{safe}_{ts}")
    }

    async fn create_healthcheck_topic_with_retry(
        ops: &dyn KafkaHealthcheckOps,
        bootstrap: &str,
        topic: &str,
        timeout: Duration,
    ) -> Result<(), String> {
        #[cfg(test)]
        let poll_every = Duration::from_millis(1);
        #[cfg(not(test))]
        let poll_every = Duration::from_millis(250);
        let start = Instant::now();

        loop {
            if start.elapsed() >= timeout {
                return Err(format!("kafka healthcheck topic create timed out: {topic}"));
            }

            match ops.create_topic(bootstrap, topic).await {
                Ok(()) => return Ok(()),
                Err(err) => log::debug!("[Kafka] healthcheck topic create failed: {err}"),
            }

            Delay::new(poll_every).await;
        }
    }

    async fn healthcheck_roundtrip(&self, bootstrap: &str) -> Result<(), String> {
        let topic = Self::healthcheck_topic_name(&self.identifier);

        Self::create_healthcheck_topic_with_retry(
            self.healthcheck_ops.as_ref(),
            bootstrap,
            &topic,
            Duration::from_secs(10),
        )
        .await?;

        let payload = format!("arena-healthcheck-{topic}");

        if let Err(err) = self
            .healthcheck_ops
            .publish(bootstrap, &topic, &payload)
            .await
        {
            let _ = self.healthcheck_ops.delete_topic(bootstrap, &topic).await;
            return Err(err);
        }

        let saw = match self
            .healthcheck_ops
            .consume_verify(bootstrap, &topic, &payload)
            .await
        {
            Ok(v) => v,
            Err(err) => {
                let _ = self.healthcheck_ops.delete_topic(bootstrap, &topic).await;
                return Err(err);
            }
        };

        let _ = self.healthcheck_ops.delete_topic(bootstrap, &topic).await;

        if !saw {
            return Err("kafka healthcheck did not observe published message".to_string());
        }

        Ok(())
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

        match self.healthcheck_roundtrip(&bootstrap).await {
            Ok(()) => {}
            Err(err) => panic!("[Kafka-{}] readiness check failed: {}", self.identifier, err),
        }
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