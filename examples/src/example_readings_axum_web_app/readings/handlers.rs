use std::time::Duration;

use arena_mssql::Client as MssqlClient;
use arena_oauth::AccessTokenClaims;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::Json;
use rskafka::client::partition::Compression;
use rskafka::record::Record;
use tokio::sync::Mutex;

use crate::example_readings_axum_web_app::state::AppState;

use super::requests::CreateReadingRequest;
use super::responses::{
    CalibrationResponse, CreateReadingResponse, ReadingCreatedEvent, ReadingRow,
};

const KAFKA_PUBLISH_TIMEOUT_MS: u64 = 500;

pub async fn list_readings(
    State(st): State<AppState>,
    Extension(claims): Extension<AccessTokenClaims>,
) -> Result<Json<Vec<ReadingRow>>, (StatusCode, String)> {
    tracing::debug!(
        subject = %claims.sub,
        phase = "list_readings",
        "handling list readings",
    );
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

async fn write_validation_result(
    mssql: &Mutex<MssqlClient>,
    user_name: &str,
    value: i32,
    valid: bool,
) -> Result<(), String> {
    let mut client = mssql.lock().await;
    client
        .execute(
            "INSERT INTO dbo.validation_results (user_name, value, valid) VALUES (@P1, @P2, @P3);",
            &[&user_name, &value, &valid],
        )
        .await
        .map_err(|e| format!("mssql insert validation_results failed: {}", e))?;
    Ok(())
}

pub async fn create_reading(
    State(st): State<AppState>,
    Extension(claims): Extension<AccessTokenClaims>,
    Json(req): Json<CreateReadingRequest>,
) -> Result<(StatusCode, Json<CreateReadingResponse>), (StatusCode, String)> {
    tracing::debug!(
        subject = %claims.sub,
        phase = "create_reading",
        "handling create reading",
    );
    let validation = st
        .http_client
        .post(format!("{}/api/v1/validate", st.calibration_url))
        .json(&serde_json::json!({ "value": req.value }))
        .send()
        .await
        .map_err(internal_error)?
        .json::<CalibrationResponse>()
        .await
        .map_err(internal_error)?;

    write_validation_result(&st.mssql, &req.user_name, req.value, validation.valid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

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
    let partition = st.kafka.clone();
    let record = Record {
        key: Some(key.into_bytes()),
        value: Some(payload.into_bytes()),
        headers: Default::default(),
        timestamp: rskafka::chrono::Utc::now(),
    };
    tokio::spawn(async move {
        match tokio::time::timeout(
            Duration::from_millis(KAFKA_PUBLISH_TIMEOUT_MS),
            partition.produce(vec![record], Compression::NoCompression),
        )
        .await
        {
            Err(_elapsed) => tracing::debug!(phase = "kafka_publish", "kafka publish timed out"),
            Ok(Err(e)) => tracing::debug!(error = %e, phase = "kafka_publish", "kafka publish failed"),
            Ok(Ok(_offsets)) => {}
        }
    });

    Ok((
        StatusCode::OK,
        Json(CreateReadingResponse {
            valid: validation.valid,
            id: Some(id),
        }),
    ))
}

fn internal_error<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
