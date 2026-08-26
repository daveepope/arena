use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use rskafka::client::partition::Compression;
use rskafka::client::Client;
use rskafka::record::Record;
use std::time::Duration;
use tokio::time::Instant;

use super::client::{connect_client, consume_until, partition_client_for};
use super::topic_creator::TopicCreator;

const DELETE_TOPIC_TIMEOUT_MS: i32 = 500;
const PUBLISH_TIMEOUT_MS: u64 = 500;
const CONSUME_WINDOW_MS: u64 = 500;
const FETCH_MAX_WAIT_MS: i32 = 50;
const RETRY_INTERVAL_MS: u64 = 50;

#[async_trait]
pub trait KafkaHealthcheckOps: Send + Sync {
    async fn create_topic(&self, client: &Client, topic: &str) -> Result<(), String>;
    async fn delete_topic(&self, client: &Client, topic: &str) -> Result<(), String>;
    async fn publish(&self, client: &Client, topic: &str, payload: &str) -> Result<(), String>;
    async fn consume_verify(
        &self,
        client: &Client,
        topic: &str,
        expected_payload: &str,
    ) -> Result<bool, String>;
}

pub(super) struct RskafkaHealthcheckOps;

pub(super) struct DefaultKafkaReadinessCheck;

#[async_trait]
impl ReadinessCheck for DefaultKafkaReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        bootstrap_servers: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let ops = RskafkaHealthcheckOps;
        run_with_retry(
            &ops,
            identifier,
            bootstrap_servers,
            Duration::from_millis(timeout_ms),
        )
        .await
    }
}

fn healthcheck_topic_name(identifier: &str) -> String {
    let safe = arena_container::identifier::sanitize_for_container(identifier).replace('-', "_");
    format!("arena_healthcheck_{safe}")
}

#[async_trait]
impl KafkaHealthcheckOps for RskafkaHealthcheckOps {
    async fn create_topic(&self, client: &Client, topic: &str) -> Result<(), String> {
        TopicCreator::create_topic(client, topic).await
    }

    async fn delete_topic(&self, client: &Client, topic: &str) -> Result<(), String> {
        let controller = client
            .controller_client()
            .map_err(|e| format!("create kafka controller client failed: {e}"))?;
        controller
            .delete_topic(topic, DELETE_TOPIC_TIMEOUT_MS)
            .await
            .map_err(|e| format!("kafka topic delete failed: {e}"))
    }

    async fn publish(&self, client: &Client, topic: &str, payload: &str) -> Result<(), String> {
        let partition = partition_client_for(client, topic).await?;

        let record = Record {
            key: Some(b"healthcheck".to_vec()),
            value: Some(payload.as_bytes().to_vec()),
            headers: Default::default(),
            timestamp: chrono::Utc::now(),
        };

        tokio::time::timeout(
            Duration::from_millis(PUBLISH_TIMEOUT_MS),
            partition.produce(vec![record], Compression::NoCompression),
        )
        .await
        .map_err(|_elapsed| "kafka publish timed out".to_string())?
        .map_err(|e| format!("kafka publish failed: {e}"))?;

        Ok(())
    }

    async fn consume_verify(
        &self,
        client: &Client,
        topic: &str,
        expected_payload: &str,
    ) -> Result<bool, String> {
        let partition = partition_client_for(client, topic).await?;
        let expected = expected_payload.as_bytes();
        let deadline = Instant::now() + Duration::from_millis(CONSUME_WINDOW_MS);

        let found = consume_until(&partition, FETCH_MAX_WAIT_MS, deadline, |r| {
            Ok((r.record.value.as_deref() == Some(expected)).then_some(()))
        })
        .await?;

        Ok(found.is_some())
    }
}

async fn roundtrip_once(
    ops: &impl KafkaHealthcheckOps,
    client: &Client,
    topic: &str,
) -> Result<(), String> {
    let payload = format!("arena-healthcheck-{topic}");
    ops.publish(client, topic, &payload).await?;

    let saw = ops.consume_verify(client, topic, &payload).await?;
    if !saw {
        return Err("kafka healthcheck did not observe published message".to_string());
    }

    Ok(())
}

async fn run_with_retry(
    ops: &impl KafkaHealthcheckOps,
    identifier: &str,
    bootstrap: &str,
    timeout: Duration,
) -> Result<(), String> {
    let topic = healthcheck_topic_name(identifier);
    let client = connect_client(bootstrap).await?;

    let result = match ops.create_topic(&client, &topic).await {
        Err(e) => Err(e),
        Ok(()) => {
            let poll_every = Duration::from_millis(RETRY_INTERVAL_MS);
            let start = Instant::now();
            loop {
                if start.elapsed() >= timeout {
                    break Err(format!("kafka healthcheck timed out (topic={topic})"));
                }

                match roundtrip_once(ops, &client, &topic).await {
                    Ok(()) => break Ok(()),
                    Err(err) => {
                        tracing::debug!(
                            subsystem = "kafka",
                            error = %err,
                            "readiness probe failed (will retry)"
                        );
                        tokio::time::sleep(poll_every).await;
                    }
                }
            }
        }
    };

    if let Err(e) = ops.delete_topic(&client, &topic).await {
        tracing::debug!(
            subsystem = "kafka",
            topic = %topic,
            error = %e,
            "healthcheck topic cleanup failed (best effort)"
        );
    }

    result
}
