use arena::{ClosedArena, Component, Dependency, Encounter, EncounterTrait, ExecutableComponent};
use arena_kafka::{KafkaDependency, KafkaFlavor};
use arena_postgres::PostgresDependency;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get},
    Json, Router,
};
use env_logger::Env;
use serde::{Deserialize, Serialize};
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio_postgres::{Client, NoTls};
use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;

#[derive(Clone)]
struct AppState {
    pg: Arc<Client>,
    kafka: Arc<BaseProducer>,
    kafka_topic: Arc<str>,
}

#[derive(Deserialize)]
struct CreateReadingRequest {
    user_name: String,
    value: i32,
    comment: Option<String>,
}

#[derive(Serialize)]
struct CreateReadingResponse {
    id: i64,
}

#[derive(Serialize)]
struct ReadingRow {
    id: i64,
    user_name: String,
    value: i32,
    comment: Option<String>,
}

#[derive(Serialize)]
struct ReadingCreatedEvent<'a> {
    id: i64,
    user_name: &'a str,
    value: i32,
    comment: &'a Option<String>,
}

async fn health() -> &'static str {
    "ok"
}

async fn log_requests(req: Request<Body>, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let sw = Instant::now();

    let res = next.run(req).await;

    log::debug!("{} {} -> {} in {:?}", method, uri, res.status(), sw.elapsed());
    res
}

async fn list_readings(
    State(st): State<AppState>,
) -> Result<Json<Vec<ReadingRow>>, (StatusCode, String)> {
    let rows = st
        .pg
        .query(
            r#"
            select r.id, u.name, r.value, r.comment
            from instrument_reading.reading r
            join instrument_reading."user" u on u.id = r."userId"
            order by r.id desc
            limit 50
            "#,
            &[],
        )
        .await
        .map_err(internal_error)?;

    let out = rows
        .into_iter()
        .map(|r| ReadingRow {
            id: r.get::<_, i64>(0),
            user_name: r.get::<_, String>(1),
            value: r.get::<_, i32>(2),
            comment: r.get::<_, Option<String>>(3),
        })
        .collect();

    Ok(Json(out))
}

async fn create_reading(
    State(st): State<AppState>,
    Json(req): Json<CreateReadingRequest>,
) -> Result<(StatusCode, Json<CreateReadingResponse>), (StatusCode, String)> {
    let user_id: i64 = match st
        .pg
        .query_opt(
            r#"select id from instrument_reading."user" where name = $1"#,
            &[&req.user_name],
        )
        .await
        .map_err(internal_error)?
    {
        Some(row) => row.get(0),
        None => {
            let row = st
                .pg
                .query_one(
                    r#"insert into instrument_reading."user"(name) values ($1) returning id"#,
                    &[&req.user_name],
                )
                .await
                .map_err(internal_error)?;
            row.get(0)
        }
    };

    let row = st
        .pg
        .query_one(
            r#"
            insert into instrument_reading.reading("userId", value, comment)
            values ($1, $2, $3)
            returning id
            "#,
            &[&user_id, &req.value, &req.comment],
        )
        .await
        .map_err(internal_error)?;

    let id: i64 = row.get(0);

    let payload = serde_json::to_string(&ReadingCreatedEvent {
        id,
        user_name: &req.user_name,
        value: req.value,
        comment: &req.comment,
    })
    .map_err(internal_error)?;

    let key = id.to_string();
    let producer = st.kafka.clone();
    let topic = st.kafka_topic.to_string();
    let payload_for_send = payload.clone();
    let key_for_send = key.clone();
    tokio::task::spawn_blocking(move || {
        let record = BaseRecord::to(topic.as_str())
            .key(key_for_send.as_str())
            .payload(payload_for_send.as_bytes());
        if let Err((e, _msg)) = producer.send(record) {
            log::debug!("kafka publish failed: {e}");
            return;
        }
        if let Err(e) = producer.flush(Duration::from_secs(2)) {
            log::debug!("kafka flush failed: {e}");
        }
    });

    Ok((StatusCode::CREATED, Json(CreateReadingResponse { id })))
}

fn internal_error<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
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

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        Env::default().default_filter_or(
            "arena=debug,arena_examples=debug,arena_postgres=debug,arena_kafka=debug,testcontainers=info,testcontainers_modules=info",
        ),
    )
    .init();

    let postgres_port = 4444u16;
    let db_name = "my_database";
    let db_user = "my_user";
    let db_pass = "my_password";

    let startup_sql_scripts =
        vec![include_str!("../resources/instrument_reading_db_schema.sql").to_string()];

    let postgres_db: Dependency = Box::new(
        PostgresDependency::builder("arena example database")
            .with_image("14.20-trixie")
            .with_port(postgres_port)
            .with_database_name(db_name)
            .with_database_username(db_user)
            .with_database_password(db_pass)
            .with_startup_sql_scripts(startup_sql_scripts)
            .build(),
    );

    let kafka: Dependency = Box::new(
        KafkaDependency::builder("arena example kafka")
            .with_flavor(KafkaFlavor::ApacheNative)
            .build(),
    );

    let dependencies: Vec<Dependency> = vec![postgres_db, kafka];

    let components: Vec<Component> = vec![Box::new(
        ExecutableComponent::builder("arena example web app").build(),
    )];

    let encounter = Encounter::new("End to end happy path", dependencies, components);
    let encounters: Vec<Box<dyn EncounterTrait>> = vec![Box::new(encounter)];
    let closed = ClosedArena::new(String::from("Example Arena"), encounters);

    let open = closed.open().await;

    let kafka_bootstrap = open
        .dependency("arena example kafka")
        .and_then(|d| d.as_any().downcast_ref::<KafkaDependency>())
        .and_then(|k| k.bootstrap_servers())
        .expect("kafka dependency bootstrap should be available after open()")
        .to_string();

    let conn_str = open
        .dependency("arena example database")
        .and_then(|d| d.as_any().downcast_ref::<PostgresDependency>())
        .and_then(|p| p.connection_string())
        .expect("postgres dependency connection string should be available after open()")
        .to_string();

    let (pg, connection) = tokio_postgres::connect(&conn_str, NoTls)
        .await
        .expect("connect to postgres");
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            log::error!("postgres connection error: {e}");
        }
    });

    let kafka_topic: Arc<str> = Arc::from("readings");
    create_topic_with_retry(&kafka_bootstrap, kafka_topic.as_ref(), Duration::from_secs(10)).await;

    let kafka: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", &kafka_bootstrap)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("create kafka producer");

    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", &kafka_bootstrap)
        .set("group.id", "arena-examples")
        .set("enable.partition.eof", "false")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("create kafka consumer");
    consumer
        .subscribe(&[kafka_topic.as_ref()])
        .expect("subscribe");

    let consumer_topic = kafka_topic.clone();
    tokio::spawn(async move {
        tokio::task::spawn_blocking(move || loop {
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
                    log::debug!("kafka received {}: {}", consumer_topic.as_ref(), payload);
                }
            }
        });
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/readings", get(list_readings).post(create_reading))
        .layer(axum::middleware::from_fn(log_requests))
        .with_state(AppState {
            pg: Arc::new(pg),
            kafka: Arc::new(kafka),
            kafka_topic,
        });

    let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    log::info!("listening on http://{addr}");

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });

    tokio::signal::ctrl_c().await.unwrap();
    let _ = shutdown_tx.send(());
    let _ = server.await;

    let _closed = open.close().await;
}