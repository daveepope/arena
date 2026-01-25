use arena::dependency::RunnableDependency;
use arena_kafka::{KafkaDependency, KafkaFlavor};
use futures::FutureExt;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct TestContext {
    kafka: KafkaDependency,
    bootstrap: String,
    admin: AdminClient<rdkafka::client::DefaultClientContext>,
    topic: String,
}

impl TestContext {
    async fn new() -> Result<Self, String> {
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

        let admin: AdminClient<_> = match ClientConfig::new()
            .set("bootstrap.servers", &bootstrap)
            .create()
        {
            Ok(v) => v,
            Err(e) => {
                kafka.stop().await;
                return Err(format!("create kafka admin client failed: {e}"));
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
        self.kafka.stop().await;
    }
}

#[tokio::test]
async fn kafka_dependency_component_happy_path_pub_sub() {
    let _ = env_logger::builder().is_test(true).try_init();
    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    let test_body = {
        let ctx_ref = &ctx;
        let bootstrap = ctx.bootstrap.clone();
        let topic = ctx.topic.clone();

        async move {
            ctx_ref.create_topic().await?;

            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let actual_payload = format!("hello-from-component-test-{ts}");

            let producer: FutureProducer = ClientConfig::new()
                .set("bootstrap.servers", &bootstrap)
                .set("message.timeout.ms", "5000")
                .create()
                .map_err(|e| format!("create kafka producer failed: {e}"))?;

            let record = FutureRecord::to(&topic)
                .key("component-test")
                .payload(&actual_payload);
            producer
                .send(record, Duration::from_secs(5))
                .await
                .map_err(|(e, _msg)| format!("produce failed: {e}"))?;

            let consumer: BaseConsumer = ClientConfig::new()
                .set("bootstrap.servers", &bootstrap)
                .set("group.id", format!("arena-component-test-{ts}"))
                .set("enable.auto.commit", "false")
                .set("auto.offset.reset", "earliest")
                .create()
                .map_err(|e| format!("create kafka consumer failed: {e}"))?;

            consumer
                .subscribe(&[&topic])
                .map_err(|e| format!("subscribe failed: {e}"))?;

            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            while std::time::Instant::now() < deadline {
                match consumer.poll(Duration::from_millis(250)) {
                    None => {}
                    Some(Err(e)) => return Err(format!("consume failed: {e}")),
                    Some(Ok(msg)) => {
                        if let Some(bytes) = msg.payload() {
                            if bytes == actual_payload.as_bytes() {
                                return Ok(());
                            }
                        }
                    }
                }
            }

            Err("did not observe produced message before timeout".to_string())
        }
    };

    let outcome = std::panic::AssertUnwindSafe(test_body).catch_unwind().await;

    ctx.delete_topic_best_effort().await;
    ctx.stop().await;

    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}