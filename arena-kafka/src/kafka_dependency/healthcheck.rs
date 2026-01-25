use async_trait::async_trait;
use arena::healthcheck::ReadinessCheck;
use futures::channel::oneshot;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub trait KafkaHealthcheckOps: Send + Sync {
    fn create_topic(&self, bootstrap: &str, topic: &str) -> Result<(), String>;
    fn delete_topic(&self, bootstrap: &str, topic: &str) -> Result<(), String>;
    fn publish(&self, bootstrap: &str, topic: &str, payload: &str) -> Result<(), String>;
    fn consume_verify(
        &self,
        bootstrap: &str,
        topic: &str,
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
            let res = run_with_retry_blocking(&ops, &identifier, &bootstrap, timeout);
            let _ = tx.send(res);
        });

        rx.await.map_err(|_canceled| "kafka healthcheck worker thread unexpectedly stopped".to_string())?
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

impl KafkaHealthcheckOps for RdkafkaHealthcheckOps {
    fn create_topic(&self, bootstrap: &str, topic: &str) -> Result<(), String> {
        let admin: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .set("log_level", "0")
            .create()
            .map_err(|e| format!("create kafka admin client failed: {e}"))?;

        let new_topic = NewTopic::new(topic, 1, TopicReplication::Fixed(1));
        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(2)));

        match futures::executor::block_on(admin.create_topics([&new_topic], &opts)) {
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

    fn delete_topic(&self, bootstrap: &str, topic: &str) -> Result<(), String> {
        let admin: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .set("log_level", "0")
            .create()
            .map_err(|e| format!("create kafka admin client failed: {e}"))?;

        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(2)));
        futures::executor::block_on(admin.delete_topics(&[topic], &opts))
            .map(|_res| ())
            .map_err(|e| format!("kafka topic delete failed: {e}"))
    }

    fn publish(&self, bootstrap: &str, topic: &str, payload: &str) -> Result<(), String> {
        let producer: BaseProducer = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .set("log_level", "0")
            .set("message.timeout.ms", "2000")
            .create()
            .map_err(|e| format!("create kafka producer failed: {e}"))?;

        let record = BaseRecord::to(topic)
            .key("healthcheck")
            .payload(payload.as_bytes());

        producer
            .send(record)
            .map_err(|(e, _msg)| format!("kafka publish failed: {e}"))?;

        producer
            .flush(Duration::from_secs(2))
            .map_err(|e| format!("kafka publish flush failed: {e}"))
    }

    fn consume_verify(
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

fn roundtrip_once_blocking(
    ops: &impl KafkaHealthcheckOps,
    bootstrap: &str,
    topic: &str,
) -> Result<(), String> {
    ops.create_topic(bootstrap, topic)?;

    let payload = format!("arena-healthcheck-{topic}");
    ops.publish(bootstrap, topic, &payload)?;

    let saw = ops.consume_verify(bootstrap, topic, &payload)?;
    if !saw {
        return Err("kafka healthcheck did not observe published message".to_string());
    }

    Ok(())
}

fn run_with_retry_blocking(
    ops: &impl KafkaHealthcheckOps,
    identifier: &str,
    bootstrap: &str,
    timeout: Duration,
) -> Result<(), String> {
    let topic = healthcheck_topic_name(identifier);

    #[cfg(test)]
    let poll_every = Duration::from_millis(1);
    #[cfg(not(test))]
    let poll_every = Duration::from_millis(100);

    let start = Instant::now();
    loop {
        if start.elapsed() >= timeout {
            return Err(format!("kafka healthcheck timed out (topic={topic})"));
        }

        let res = roundtrip_once_blocking(ops, bootstrap, &topic);

        let _ = ops.delete_topic(bootstrap, &topic);

        match res {
            Ok(()) => return Ok(()),
            Err(err) => {
                log::debug!("[Kafka] healthcheck failed (will retry): {err}");
                std::thread::sleep(poll_every);
            }
        }
    }
}

