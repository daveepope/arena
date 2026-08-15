use rskafka::client::partition::{OffsetAt, PartitionClient, UnknownTopicHandling};
use rskafka::client::{Client, ClientBuilder};
use rskafka::record::RecordAndOffset;
use rskafka::BackoffConfig;
use std::time::Duration;
use tokio::time::Instant;

pub const KAFKA_TOPIC_PARTITION: i32 = 0;
pub const CONNECT_DEADLINE_MS: u64 = 2000;
const FETCH_MIN_BYTES: i32 = 1;
const FETCH_MAX_BYTES: i32 = 1_000_000;

pub async fn connect_client(bootstrap: &str) -> Result<Client, String> {
    ClientBuilder::new(vec![bootstrap.to_string()])
        .backoff_config(BackoffConfig {
            deadline: Some(Duration::from_millis(CONNECT_DEADLINE_MS)),
            ..Default::default()
        })
        .build()
        .await
        .map_err(|e| format!("create kafka client failed: {e}"))
}

pub async fn partition_client_for(
    client: &Client,
    topic: &str,
) -> Result<PartitionClient, String> {
    client
        .partition_client(topic, KAFKA_TOPIC_PARTITION, UnknownTopicHandling::Retry)
        .await
        .map_err(|e| format!("create kafka partition client failed for topic {topic}: {e}"))
}

pub async fn partition_client_for_existing(
    client: &Client,
    topic: &str,
) -> Result<PartitionClient, String> {
    client
        .partition_client(topic, KAFKA_TOPIC_PARTITION, UnknownTopicHandling::Error)
        .await
        .map_err(|e| format!("create kafka partition client failed for topic {topic}: {e}"))
}

pub async fn consume_until<T>(
    partition: &PartitionClient,
    max_wait_ms: i32,
    deadline: Instant,
    mut extract: impl FnMut(&RecordAndOffset) -> Result<Option<T>, String>,
) -> Result<Option<T>, String> {
    let mut next_offset = partition
        .get_offset(OffsetAt::Earliest)
        .await
        .map_err(|e| format!("get kafka earliest offset failed: {e}"))?;

    while Instant::now() < deadline {
        let (records, _high_watermark) = partition
            .fetch_records(next_offset, FETCH_MIN_BYTES..FETCH_MAX_BYTES, max_wait_ms)
            .await
            .map_err(|e| format!("kafka consume failed: {e}"))?;

        for r in &records {
            if let Some(v) = extract(r)? {
                return Ok(Some(v));
            }
        }
        if let Some(last) = records.last() {
            next_offset = last.offset + 1;
        }
    }

    Ok(None)
}
