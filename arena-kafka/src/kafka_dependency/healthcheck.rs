use async_trait::async_trait;
use arena::healthcheck::ReadinessCheck;
use futures::channel::oneshot;
use rdkafka::admin::{AdminClient, AdminOptions};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
use std::time::{Duration, Instant};

use super::topic_creator::TopicCreator;

pub trait KafkaHealthcheckOps: Send + Sync {
    fn create_topic(&self, bootstrap: &str, topic: &str) -> Result<(), String>;
    fn delete_topic(&self, bootstrap: &str, topic: &str) -> Result<(), String>;
    fn publish(&self, bootstrap: &str, topic: &str, payload: &str) -> Result<(), String>;
    fn create_consumer(&self, bootstrap: &str, topic: &str) -> Result<BaseConsumer, String>;
    fn consume_verify(
        &self,
        consumer: &BaseConsumer,
        expected_payload: &str,
    ) -> Result<bool, String>;
}

pub(super) struct RdkafkaHealthcheckOps;

pub(super) struct DefaultKafkaReadinessCheck;

#[async_trait]
impl ReadinessCheck for DefaultKafkaReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        bootstrap_servers: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let identifier = identifier.to_string();
        let bootstrap = bootstrap_servers.to_string();
        let timeout = Duration::from_millis(timeout_ms);
        let (tx, rx) = oneshot::channel::<Result<(), String>>();

        std::thread::spawn(move || {
            let ops = RdkafkaHealthcheckOps;
            let res = run_with_retry(&ops, &identifier, &bootstrap, timeout);
            let _ = tx.send(res);
        });

        rx.await.map_err(|_canceled| "kafka healthcheck worker thread unexpectedly stopped".to_string())?
    }
}

fn healthcheck_topic_name(identifier: &str) -> String {
    let safe = super::sanitize_identifier(identifier);
    format!("arena_healthcheck_{safe}")
}

impl KafkaHealthcheckOps for RdkafkaHealthcheckOps {
    fn create_topic(&self, bootstrap: &str, topic: &str) -> Result<(), String> {
        TopicCreator::create_topic(bootstrap, topic)
    }

    fn delete_topic(&self, bootstrap: &str, topic: &str) -> Result<(), String> {
        let admin: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .set("log_level", "0")
            .create()
            .map_err(|e| format!("create kafka admin client failed: {e}"))?;

        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_millis(1000)));
        futures::executor::block_on(admin.delete_topics(&[topic], &opts))
            .map(|_res| ())
            .map_err(|e| format!("kafka topic delete failed: {e}"))
    }

    fn publish(&self, bootstrap: &str, topic: &str, payload: &str) -> Result<(), String> {
        let producer: BaseProducer = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .set("log_level", "0")
            .set("message.timeout.ms", "1000")
            .create()
            .map_err(|e| format!("create kafka producer failed: {e}"))?;

        let record = BaseRecord::to(topic)
            .key("healthcheck")
            .payload(payload.as_bytes());

        producer
            .send(record)
            .map_err(|(e, _msg)| format!("kafka publish failed: {e}"))?;

        producer
            .flush(Duration::from_millis(1000))
            .map_err(|e| format!("kafka publish flush failed: {e}"))
    }

    fn create_consumer(&self, bootstrap: &str, topic: &str) -> Result<BaseConsumer, String> {
        let consumer: BaseConsumer = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .set("log_level", "0")
            .set("group.id", format!("arena-healthcheck-{topic}"))
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create()
            .map_err(|e| format!("create kafka consumer failed: {e}"))?;

        consumer
            .subscribe(&[topic])
            .map_err(|e| format!("kafka subscribe failed: {e}"))?;

        Ok(consumer)
    }

    fn consume_verify(
        &self,
        consumer: &BaseConsumer,
        expected_payload: &str,
    ) -> Result<bool, String> {
        let consume_deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < consume_deadline {
            match consumer.poll(Duration::from_millis(10)) {
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

fn roundtrip_once(
    ops: &impl KafkaHealthcheckOps,
    consumer: &BaseConsumer,
    bootstrap: &str,
    topic: &str,
) -> Result<(), String> {
    let payload = format!("arena-healthcheck-{topic}");
    ops.publish(bootstrap, topic, &payload)?;

    let saw = ops.consume_verify(consumer, &payload)?;
    if !saw {
        return Err("kafka healthcheck did not observe published message".to_string());
    }

    Ok(())
}

struct HealthcheckTopicHandle<'a> {
    ops: &'a dyn KafkaHealthcheckOps,
    bootstrap: String,
    topic: String,
}

impl Drop for HealthcheckTopicHandle<'_> {
    fn drop(&mut self) {
        let _ = self.ops.delete_topic(&self.bootstrap, &self.topic);
    }
}

fn run_with_retry(
    ops: &impl KafkaHealthcheckOps,
    identifier: &str,
    bootstrap: &str,
    timeout: Duration,
) -> Result<(), String> {
    let topic = healthcheck_topic_name(identifier);

    let _topic_handle = HealthcheckTopicHandle {
        ops,
        bootstrap: bootstrap.to_string(),
        topic: topic.clone(),
    };

    ops.create_topic(bootstrap, &topic)?;
    let consumer = ops.create_consumer(bootstrap, &topic)?;

    #[cfg(test)]
    let poll_every = Duration::from_millis(1);
    #[cfg(not(test))]
    let poll_every = Duration::from_millis(50);

    let start = Instant::now();
    loop {
        if start.elapsed() >= timeout {
            return Err(format!("kafka healthcheck timed out (topic={topic})"));
        }

        match roundtrip_once(ops, &consumer, bootstrap, &topic) {
            Ok(()) => return Ok(()),
            Err(err) => {
                log::debug!("[Kafka] healthcheck failed (will retry): {err}");
                std::thread::sleep(poll_every);
            }
        }
    }
}

