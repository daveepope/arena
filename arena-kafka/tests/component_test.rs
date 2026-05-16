use arena::dependency::RunnableDependency;
use arena_kafka::{KafkaDependency, KafkaFlavor, TopicCreator};
use futures::FutureExt;
use rdkafka::admin::{AdminClient, AdminOptions};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RDKAFKA_LOG_LEVEL_SILENT: &str = "0";

fn init_test_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
}

fn new_admin(
    bootstrap: &str,
) -> Result<AdminClient<rdkafka::client::DefaultClientContext>, String> {
    ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("log_level", RDKAFKA_LOG_LEVEL_SILENT)
        .create()
        .map_err(|e| format!("create kafka admin client failed: {e}"))
}

fn new_producer(bootstrap: &str) -> Result<BaseProducer, String> {
    ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("log_level", RDKAFKA_LOG_LEVEL_SILENT)
        .set("message.timeout.ms", "2000")
        .create()
        .map_err(|e| format!("create kafka producer failed: {e}"))
}

fn new_consumer(bootstrap: &str, group_id: &str) -> Result<BaseConsumer, String> {
    ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("log_level", RDKAFKA_LOG_LEVEL_SILENT)
        .set("group.id", group_id)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .create()
        .map_err(|e| format!("create kafka consumer failed: {e}"))
}

fn spawn_poll_until_payload_observed(
    consumer: BaseConsumer,
    expected_payload: String,
    timeout: Duration,
) -> tokio::task::JoinHandle<Result<(), String>> {
    tokio::task::spawn_blocking(move || {
        let expected_bytes = expected_payload.into_bytes();
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            match consumer.poll(Duration::from_millis(10)) {
                None => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Some(Err(e)) => return Err(format!("consume failed: {e}")),
                Some(Ok(msg)) => {
                    if let Some(bytes) = msg.payload() {
                        if bytes == expected_bytes.as_slice() {
                            return Ok(());
                        }
                    }
                }
            }
        }
        Err("did not observe produced message before timeout".to_string())
    })
}

fn produce_payload_once(producer: &BaseProducer, topic: &str, payload: &str) -> Result<(), String> {
    let record = BaseRecord::to(topic)
        .key("component-test")
        .payload(payload.as_bytes());
    producer
        .send(record)
        .map_err(|(e, _msg)| format!("produce failed: {e}"))?;
    producer
        .flush(Duration::from_secs(2))
        .map_err(|e| format!("produce flush failed: {e}"))
}

struct TestContext {
    kafka: KafkaDependency,
    bootstrap: String,
    admin: AdminClient<rdkafka::client::DefaultClientContext>,
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

        let admin = match new_admin(&bootstrap) {
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
            bootstrap,
            admin,
            topic,
        })
    }

    fn create_topic(&self) -> Result<(), String> {
        TopicCreator::create_topic(&self.bootstrap, &self.topic)
    }

    async fn delete_topic_best_effort(&self) {
        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(2)));
        let _ = self
            .admin
            .delete_topics(&[self.topic.as_str()], &opts)
            .await;
    }

    async fn stop(mut self) {
        drop(self.admin);
        self.kafka.stop().await;
    }
}

async fn assert_pub_sub_roundtrip(ctx: &TestContext) -> Result<(), String> {
    let sw = std::time::Instant::now();
    ctx.create_topic()?;
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
    let group_id = format!("arena-component-test-{ts}");

    let sw = std::time::Instant::now();
    let consumer = new_consumer(&ctx.bootstrap, &group_id)?;
    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_kafka",
        step = "new_consumer",
        elapsed = ?sw.elapsed(),
        "timing checkpoint",
    );

    let sw = std::time::Instant::now();
    consumer
        .subscribe(&[&ctx.topic])
        .map_err(|e| format!("subscribe failed: {e}"))?;
    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_kafka",
        step = "subscribe",
        elapsed = ?sw.elapsed(),
        "timing checkpoint",
    );

    let sw = std::time::Instant::now();
    for _ in 0..30 {
        consumer.poll(Duration::from_millis(100));
    }
    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_kafka",
        step = "consumer_warmup",
        elapsed = ?sw.elapsed(),
        "timing checkpoint",
    );

    let poll_handle =
        spawn_poll_until_payload_observed(consumer, payload.clone(), Duration::from_secs(5));

    let sw = std::time::Instant::now();
    let producer = new_producer(&ctx.bootstrap)?;
    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_kafka",
        step = "new_producer",
        elapsed = ?sw.elapsed(),
        "timing checkpoint",
    );

    let topic = ctx.topic.clone();
    let payload_for_produce = payload.clone();
    let sw = std::time::Instant::now();
    tokio::task::spawn_blocking(move || {
        produce_payload_once(&producer, topic.as_str(), payload_for_produce.as_str())
    })
    .await
    .map_err(|e| format!("produce task join failed: {e}"))??;
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
