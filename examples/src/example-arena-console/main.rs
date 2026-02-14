use arena::{ClosedArena, Component, Dependency, Encounter, EncounterTrait};
use arena_kafka::{KafkaDependency, KafkaFlavor};
use arena_postgres::PostgresDependency;
use env_logger::Env;
use arena_executable_component::executable_component::ExecutableComponent;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const POSTGRES_PORT: u16 = 4444;
const DB_NAME: &str = "my_database";
const DB_USER: &str = "my_user";
const DB_PASS: &str = "my_password";
const KAFKA_PORT: u16 = 9092;
const KAFKA_TOPIC: &str = "readings";
const WEB_APP_PORT: u16 = 3000;

fn setup_arena_components() -> Vec<Component> {
    vec![Box::new(
        ExecutableComponent::builder("arena example web app")
            .with_source_path("examples")
            .with_build_tool(arena_executable_component::BuildTool::Cargo)
            .with_executable_path("target/release/web-app")
            .with_env_var("RUST_LOG", "debug")
            .with_runtime_arg("web_app_port", WEB_APP_PORT.to_string())
            .with_runtime_arg(
                "postgres_connection_string",
                format!(
                    "host=localhost port={} user={} password={} dbname={}",
                    POSTGRES_PORT, DB_USER, DB_PASS, DB_NAME
                )
            )
            .with_runtime_arg("kafka_bootstrap", format!("localhost:{}", KAFKA_PORT))
            .build()
    )]
}

fn setup_arena_dependencies() -> Vec<Dependency> {
    let startup_sql_scripts =
        vec![include_str!("../../resources/instrument_reading_db_schema.sql").to_string()];

    let postgres_db: Dependency = Box::new(
        PostgresDependency::builder("arena example database")
            .with_image("14.20-trixie")
            .with_port(POSTGRES_PORT)
            .with_database_name(DB_NAME)
            .with_database_username(DB_USER)
            .with_database_password(DB_PASS)
            .with_startup_sql_scripts(startup_sql_scripts)
            .build(),
    );

    let kafka: Dependency = Box::new(
        KafkaDependency::builder("arena example kafka")
            .with_flavor(KafkaFlavor::ApacheNative)
            .with_port(KAFKA_PORT)
            .build(),
    );

    vec![postgres_db, kafka]
}

async fn create_kafka_topic(bootstrap: &str, topic: &str) {
    create_topic_with_retry(bootstrap, topic, Duration::from_secs(10)).await;
}

async fn create_topic_with_retry(bootstrap: &str, topic: &str, timeout: Duration) {
    let start = Instant::now();
    let poll_every = Duration::from_millis(250);

    loop {
        if start.elapsed() >= timeout {
            panic!("kafka topic create timed out (topic={topic})");
        }

        let admin: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", bootstrap)
            .create()
            .expect("create kafka admin client");

        let new_topic = NewTopic::new(topic, 1, TopicReplication::Fixed(1));
        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(2)));

        let ok = match admin.create_topics([&new_topic], &opts).await {
            Ok(results) => {
                let mut ok = true;
                for r in results {
                    if let Err((_t, e)) = r {
                        if e.to_string().to_lowercase().contains("already exists") {
                            ok = true;
                            break;
                        }
                        ok = false;
                        log::debug!("kafka topic create failed: {e}");
                        break;
                    }
                }
                ok
            }
            Err(err) => {
                log::debug!("kafka topic create request failed: {err}");
                false
            }
        };

        if ok {
            return;
        }

        tokio::time::sleep(poll_every).await;
    }
}

struct KafkaConsumerHandle {
    shutdown_signal: Arc<AtomicBool>,
}

impl Drop for KafkaConsumerHandle {
    fn drop(&mut self) {
        log::debug!("dropping kafka consumer handle, signaling shutdown");
        self.shutdown_signal.store(true, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn create_output_kafka_consumer(
    kafka_bootstrap: &str,
    topic: &str,
) -> KafkaConsumerHandle {
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", kafka_bootstrap)
        .set("group.id", "arena-examples")
        .set("enable.partition.eof", "false")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("create kafka consumer");
    
    consumer
        .subscribe(&[topic])
        .expect("subscribe");

    let topic = topic.to_string();
    let should_shutdown = Arc::new(AtomicBool::new(false));
    let should_shutdown_clone = should_shutdown.clone();
    
    tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            while !should_shutdown_clone.load(Ordering::Relaxed) {
                match consumer.poll(Duration::from_millis(250)) {
                    None => {}
                    Some(Err(err)) => {
                        log::debug!("kafka consume error: {err}");
                    }
                    Some(Ok(msg)) => {
                        let payload = msg
                            .payload_view::<str>()
                            .and_then(|r| r.ok())
                            .unwrap_or("");
                        log::debug!("kafka received {}: {}", topic, payload);
                    }
                }
            }
            log::debug!("kafka consumer shutting down");
        })
        .await
        .ok();
    });
    
    KafkaConsumerHandle {
        shutdown_signal: should_shutdown,
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        Env::default().default_filter_or(
            "arena=debug,arena_examples=debug,arena_postgres=debug,arena_kafka=debug,testcontainers=info,testcontainers_modules=info",
        ),
    )
    .init();

    let dependencies = setup_arena_dependencies();
    let components = setup_arena_components();
    let encounters: Vec<Box<dyn EncounterTrait>> = vec![Box::new( Encounter::new("End to end happy path encounter", dependencies, components))];
    let closed_arena = ClosedArena::new(String::from("Example Arena"), encounters);
    
    let open_arena = closed_arena.open().await;

    let kafka_bootstrap = open_arena
        .dependency("arena example kafka")
        .and_then(|d| d.as_any().downcast_ref::<KafkaDependency>())
        .and_then(|k| k.bootstrap_servers())
        .expect("kafka dependency bootstrap should be available after open()")
        .to_string();

    create_kafka_topic(&kafka_bootstrap, KAFKA_TOPIC).await;
    let _consumer_handle = create_output_kafka_consumer(&kafka_bootstrap, KAFKA_TOPIC);

    tokio::signal::ctrl_c().await.unwrap();
}