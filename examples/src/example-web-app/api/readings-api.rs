use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Json, Router,
};
use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_postgres::Client;
use serde::{Deserialize, Serialize};

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

#[derive(Clone)]
struct AppState {
    pg: Arc<Client>,
    kafka: Arc<BaseProducer>,
    kafka_topic: Arc<str>,
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

    Ok((StatusCode::OK, Json(CreateReadingResponse { id })))
}

fn internal_error<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

pub struct ExampleAxumWebApp {
    pg: Arc<Client>,
    kafka: Arc<BaseProducer>,
    kafka_topic: Arc<str>,
}

impl ExampleAxumWebApp {
    pub async fn new(
        postgres_connection_string: &str,
        kafka_bootstrap: &str,
        kafka_topic: &str,
    ) -> Self {
        use rdkafka::config::ClientConfig;
        use tokio_postgres::NoTls;

        let (pg, connection) = tokio_postgres::connect(postgres_connection_string, NoTls)
            .await
            .expect("connect to postgres");
        
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("postgres connection error: {e}");
            }
        });

        let kafka: BaseProducer = ClientConfig::new()
            .set("bootstrap.servers", kafka_bootstrap)
            .set("message.timeout.ms", "5000")
            .create()
            .expect("create kafka producer");

        Self {
            pg: Arc::new(pg),
            kafka: Arc::new(kafka),
            kafka_topic: Arc::from(kafka_topic),
        }
    }

    pub async fn serve(
        self,
        port: u16,
        shutdown_signal: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let app = Router::new()
            .route("/health", get(health))
            .route("/readings", get(list_readings).post(create_reading))
            .layer(axum::middleware::from_fn(log_requests))
            .with_state(AppState {
                pg: self.pg,
                kafka: self.kafka,
                kafka_topic: self.kafka_topic,
            });

        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
        let listener = tokio::net::TcpListener::bind(addr).await?;
        log::info!("listening on http://{addr}");

        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_signal.await;
            })
            .await?;

        Ok(())
    }
}
