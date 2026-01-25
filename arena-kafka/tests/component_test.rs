use arena::dependency::RunnableDependency;
use arena_kafka::{KafkaDependency, KafkaFlavor};
use futures::FutureExt;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RDKAFKA_LOG_LEVEL_SILENT: &str = "0";

fn init_test_logging() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .is_test(true)
        .try_init();
}

fn new_admin(bootstrap: &str) -> Result<AdminClient<rdkafka::client::DefaultClientContext>, String> {
    ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("log_level", RDKAFKA_LOG_LEVEL_SILENT)
        .create()
        .map_err(|e| format!("create kafka admin client failed: {e}"))
}

fn new_producer(bootstrap: &str) -> Result<FutureProducer, String> {
    ClientConfig::new()
        .set("bootstrap.servers", bootstrap)
        .set("log_level", RDKAFKA_LOG_LEVEL_SILENT)
        .set("message.timeout.ms", "5000")
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
                None => {}
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

async fn produce_payload_once(
    producer: &FutureProducer,
    topic: &str,
    payload: &str,
) -> Result<(), String> {
    let record = FutureRecord::to(topic)
        .key("component-test")
        .payload(payload);
    producer
        .send(record, Duration::from_secs(5))
        .await
        .map(|_delivery| ())
        .map_err(|(e, _msg)| format!("produce failed: {e}"))
}

struct TestContext {
    kafka: KafkaDependency,
    bootstrap: String,
    admin: AdminClient<rdkafka::client::DefaultClientContext>,
    topic: String,
}

impl TestContext {
    async fn new() -> Result<Self, String> {
        log::info!("[component-test] starting KafkaDependency");
        let mut kafka = KafkaDependency::builder("arena-kafka component test")
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
        log::info!("[component-test] kafka started (bootstrap={bootstrap})");

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

    async fn create_topic(&self) -> Result<(), String> {
        let new_topic = NewTopic::new(&self.topic, 1, TopicReplication::Fixed(1));
        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(5)));

        let results = self
            .admin
            .create_topics([&new_topic], &opts)
            .await
            .map_err(|e| format!("create topic request failed: {e}"))?;

        for r in results {
            if let Err((_t, e)) = r {
                return Err(format!("create topic failed: {e}"));
            }
        }

        Ok(())
    }

    async fn delete_topic_best_effort(&self) {
        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(5)));
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

/// Assert that a produced payload is observed by a subscribed consumer within a timeout.
async fn assert_pub_sub_roundtrip(ctx: &TestContext) -> Result<(), String> {
    ctx.create_topic().await?;

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let payload = format!("hello-from-component-test-{ts}");
    let group_id = format!("arena-component-test-{ts}");

    let consumer = new_consumer(&ctx.bootstrap, &group_id)?;
    consumer
        .subscribe(&[&ctx.topic])
        .map_err(|e| format!("subscribe failed: {e}"))?;

    let poll_handle =
        spawn_poll_until_payload_observed(consumer, payload.clone(), Duration::from_secs(10));

    let producer = new_producer(&ctx.bootstrap)?;
    produce_payload_once(&producer, &ctx.topic, &payload).await?;

    poll_handle.await.unwrap_or_else(|e| Err(format!("poll task join failed: {e}")))
}

#[tokio::test]
async fn kafka_dependency_lifecycle_component_test() {
    init_test_logging();
    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    log::info!("[component-test] pub/sub begin (topic={})", ctx.topic);
    let outcome = std::panic::AssertUnwindSafe(assert_pub_sub_roundtrip(&ctx))
        .catch_unwind()
        .await;

    ctx.delete_topic_best_effort().await;
    ctx.stop().await;

    match outcome {
        Ok(Ok(())) => {
            log::info!("[component-test] ok");
        }
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}