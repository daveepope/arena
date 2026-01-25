use async_trait::async_trait;
use futures_timer::Delay;
use arena::healthcheck::ReadinessCheck;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[async_trait]
pub trait KafkaHealthcheckOps: Send + Sync {
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

pub(super) struct RdkafkaHealthcheckOps;

pub(super) struct KafkaDefaultReadinessCheck;

#[async_trait]
impl ReadinessCheck for KafkaDefaultReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        bootstrap_servers: &str,
        timeout: Duration,
    ) -> Result<(), String> {
        run_with_retry(&RdkafkaHealthcheckOps, identifier, bootstrap_servers, timeout).await
    }
}

fn healthcheck_topic_name(identifier: &str) -> String {
    let safe = super::sanitize_identifier(identifier);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("arena_healthcheck_{safe}_{ts}")
}

#[async_trait]
impl KafkaHealthcheckOps for RdkafkaHealthcheckOps {
    async fn create_topic(&self, bootstrap: &str, topic: &str) -> Result<(), String> {
        let admin: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .set("log_level", "0")
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
            .set("log_level", "0")
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
            .set("log_level", "0")
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
            .set("log_level", "0")
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

async fn roundtrip_once(
    ops: &dyn KafkaHealthcheckOps,
    bootstrap: &str,
    topic: &str,
) -> Result<(), String> {
    ops.create_topic(bootstrap, topic).await?;

    let payload = format!("arena-healthcheck-{topic}");
    ops.publish(bootstrap, topic, &payload).await?;

    let saw = ops.consume_verify(bootstrap, topic, &payload).await?;
    if !saw {
        return Err("kafka healthcheck did not observe published message".to_string());
    }

    Ok(())
}

pub(crate) async fn run_with_retry(
    ops: &dyn KafkaHealthcheckOps,
    identifier: &str,
    bootstrap: &str,
    timeout: Duration,
) -> Result<(), String> {
    let topic = healthcheck_topic_name(identifier);

    #[cfg(test)]
    let poll_every = Duration::from_millis(1);
    #[cfg(not(test))]
    let poll_every = Duration::from_millis(250);

    let start = Instant::now();
    loop {
        if start.elapsed() >= timeout {
            return Err(format!("kafka healthcheck timed out (topic={topic})"));
        }

        let res = roundtrip_once(ops, bootstrap, &topic).await;

        let _ = ops.delete_topic(bootstrap, &topic).await;

        match res {
            Ok(()) => return Ok(()),
            Err(err) => {
                log::debug!("[Kafka] healthcheck failed (will retry): {err}");
                Delay::new(poll_every).await;
            }
        }
    }
}

