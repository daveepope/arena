use arena::dependency::RunnableDependency;
use arena_kafka::kafka_dependency::client::{connect_client, partition_client_for};
use arena_kafka::{KafkaDependency, KafkaFlavor, TopicCreator};
use futures::FutureExt;
use rskafka::client::partition::{Compression, PartitionClient};
use rskafka::client::Client;
use rskafka::record::Record;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const FETCH_MAX_WAIT_MS: i32 = 50;
const CONSUME_TIMEOUT_MS: u64 = 2000;
const DELETE_TOPIC_TIMEOUT_MS: i32 = 500;

fn init_test_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
}

async fn poll_until_payload_observed(
    partition: PartitionClient,
    expected_payload: String,
    timeout: Duration,
) -> Result<(), String> {
    let expected_bytes = expected_payload.into_bytes();
    let deadline = tokio::time::Instant::now() + timeout;

    let found = arena_kafka::kafka_dependency::client::consume_until(
        &partition,
        FETCH_MAX_WAIT_MS,
        deadline,
        |r| Ok((r.record.value.as_deref() == Some(expected_bytes.as_slice())).then_some(())),
    )
    .await?;

    found.ok_or_else(|| "did not observe produced message before timeout".to_string())
}

async fn produce_payload_once(partition: &PartitionClient, payload: &str) -> Result<(), String> {
    let record = Record {
        key: Some(b"component-test".to_vec()),
        value: Some(payload.as_bytes().to_vec()),
        headers: Default::default(),
        timestamp: chrono::Utc::now(),
    };
    partition
        .produce(vec![record], Compression::NoCompression)
        .await
        .map_err(|e| format!("produce failed: {e}"))?;
    Ok(())
}

struct TestContext {
    kafka: KafkaDependency,
    client: Client,
    topic: String,
}

impl TestContext {
    async fn new() -> Result<Self, String> {
        tracing::info!(
            suite = "crate_component",
            crate_under_test = "arena_kafka",
            phase = "dependency_start_begin",
            "starting dependency",
        );
        let mut kafka = KafkaDependency::builder("")
            .with_flavor(KafkaFlavor::ApacheNative)
            .build();

        kafka.start().await;

        let bootstrap = match kafka.bootstrap_servers() {
            Some(v) => v.to_string(),
            None => {
                kafka.stop().await;
                return Err("kafka bootstrap servers missing after start()".to_string());
            }
        };
        tracing::info!(
            suite = "crate_component",
            crate_under_test = "arena_kafka",
            phase = "dependency_running",
            bootstrap = %bootstrap,
            "dependency bootstrap known",
        );

        let client = match connect_client(&bootstrap).await {
            Ok(v) => v,
            Err(e) => {
                kafka.stop().await;
                return Err(e);
            }
        };

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let topic = format!("arena_component_test_{ts}");

        Ok(Self {
            kafka,
            client,
            topic,
        })
    }

    async fn create_topic(&self) -> Result<(), String> {
        TopicCreator::create_topic(&self.client, &self.topic).await
    }

    async fn delete_topic_best_effort(&self) {
        if let Ok(controller) = self.client.controller_client() {
            let _ = controller
                .delete_topic(&self.topic, DELETE_TOPIC_TIMEOUT_MS)
                .await;
        }
    }

    async fn stop(mut self) {
        self.kafka.stop().await;
    }
}

async fn assert_pub_sub_roundtrip(ctx: &TestContext) -> Result<(), String> {
    let sw = std::time::Instant::now();
    ctx.create_topic().await?;
    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_kafka",
        step = "create_topic",
        elapsed = ?sw.elapsed(),
        "timing checkpoint",
    );

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let payload = format!("hello-from-component-test-{ts}");

    let sw = std::time::Instant::now();
    let consume_partition = partition_client_for(&ctx.client, &ctx.topic).await?;
    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_kafka",
        step = "new_consumer",
        elapsed = ?sw.elapsed(),
        "timing checkpoint",
    );

    let poll_handle = tokio::spawn(poll_until_payload_observed(
        consume_partition,
        payload.clone(),
        Duration::from_millis(CONSUME_TIMEOUT_MS),
    ));

    let sw = std::time::Instant::now();
    let produce_partition = partition_client_for(&ctx.client, &ctx.topic).await?;
    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_kafka",
        step = "new_producer",
        elapsed = ?sw.elapsed(),
        "timing checkpoint",
    );

    let sw = std::time::Instant::now();
    produce_payload_once(&produce_partition, &payload).await?;
    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_kafka",
        step = "produce",
        elapsed = ?sw.elapsed(),
        "timing checkpoint",
    );

    let sw = std::time::Instant::now();
    let result = poll_handle
        .await
        .map_err(|e| format!("poll task join failed: {e}"))?;
    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_kafka",
        step = "consume",
        elapsed = ?sw.elapsed(),
        "timing checkpoint",
    );

    result
}

#[tokio::test]
async fn kafka_dependency_lifecycle_component_test() {
    init_test_logging();
    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_kafka",
        phase = "pub_sub_begin",
        topic = %ctx.topic,
        "begin pub sub roundtrip",
    );
    let outcome = std::panic::AssertUnwindSafe(assert_pub_sub_roundtrip(&ctx))
        .catch_unwind()
        .await;

    ctx.delete_topic_best_effort().await;
    ctx.stop().await;

    match outcome {
        Ok(Ok(())) => {
            tracing::info!(
                suite = "crate_component",
                crate_under_test = "arena_kafka",
                phase = "pub_sub_ok",
                "scenario passed",
            );
        }
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}
