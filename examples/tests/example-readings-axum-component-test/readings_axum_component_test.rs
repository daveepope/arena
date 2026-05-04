mod arena;
mod http;

use arena_http::{server_error, HttpDependency};
use arena_kafka::KafkaDependency;
use arena_mssql::MssqlDependency;
use arena_oauth::OauthDependency;
use std::time::Duration;

use crate::arena::{
    readings_axum_component_runtime, shared_arena, CALIBRATION_ID, EXEC_WEB_APP_PORT, KAFKA_ID,
    MSSQL_ID, OAUTH_ID, SCENARIO_LOCK, oauth_server_tls_cert_pem,
};
use crate::http::{
    consume_reading_created_event, create_reading, fetch_example_access_token,
    fetch_example_access_token_with_scope, get_readings, CreateReadingRequest,
};
use arena_examples::example_readings_axum_web_app::state::build_http_client_trusting_oauth_ca;

#[test]
fn readings_axum_exec_creates_reading_consumes_and_gets_reading() {
    readings_axum_component_runtime().block_on(async {
        let arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;

        let _oauth = arena
            .dependency(OAUTH_ID.get().expect("oauth id initialized"))
            .and_then(|d| d.as_any().downcast_ref::<OauthDependency>())
            .expect("oauth dependency");
        assert!(
            _oauth.base_url().is_some(),
            "oauth base_url should be set after arena open"
        );

        let bootstrap = arena
            .dependency(KAFKA_ID.get().expect("kafka id initialized"))
            .and_then(|d| d.as_any().downcast_ref::<KafkaDependency>())
            .and_then(|k| k.bootstrap_servers())
            .expect("kafka bootstrap should be available")
            .to_string();

        let validation_db = arena
            .dependency(MSSQL_ID.get().expect("mssql id initialized"))
            .and_then(|d| d.as_any().downcast_ref::<MssqlDependency>())
            .expect("validation database should be available");
        let validation_playbook = validation_db.playbook().run().await;

        let (id_tx, id_rx) = std::sync::mpsc::channel();
        let consume_handle = tokio::task::spawn_blocking({
            move || {
                consume_reading_created_event(
                    bootstrap,
                    "readings".to_string(),
                    id_rx,
                    Duration::from_secs(5),
                )
            }
        });

        let created_id = create_reading(
            EXEC_WEB_APP_PORT,
            "Exec Test User",
            42,
            Some("test comment".to_string()),
        )
        .await;
        id_tx
            .send(created_id as i64)
            .expect("send created_id to consumer");

        let consumed = consume_handle
            .await
            .expect("consume task join")
            .expect("should consume ReadingCreatedEvent from Kafka");

        assert_eq!(consumed.id, created_id as i64);
        assert_eq!(consumed.user_name, "Exec Test User");
        assert_eq!(consumed.value, 42);
        assert_eq!(consumed.comment.as_deref(), Some("test comment"));

        let readings = get_readings(EXEC_WEB_APP_PORT).await;
        let found = readings
            .iter()
            .find(|r| r.id == created_id)
            .expect("should find newly created reading");

        assert_eq!(found.id, created_id);
        assert_eq!(found.user_name, "Exec Test User");
        assert_eq!(found.value, 42);
        assert_eq!(found.comment.as_deref(), Some("test comment"));

        let validation_count = validation_playbook
            .verify("SELECT COUNT(*) FROM dbo.validation_results WHERE user_name = N'Exec Test User' AND value = 42 AND valid = 1;")
            .await;
        assert_eq!(
            validation_count, 1,
            "web app should have written one valid validation row to mssql"
        );
    });
}

#[test]
fn readings_axum_exec_calibration_outage_returns_error() {
    readings_axum_component_runtime().block_on(async {
        let arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;

        let calibration = arena
            .dependency(CALIBRATION_ID.get().expect("calibration id initialized"))
            .and_then(|d| d.as_any().downcast_ref::<HttpDependency>())
            .expect("calibration service should be available");

        {
            let _outage = calibration
                .playbook()
                .post("/api/v1/validate")
                .with_priority(1)
                .will_return(server_error())
                .run()
                .await;

            let token = fetch_example_access_token().await;
            let url = format!("http://127.0.0.1:{}/readings", EXEC_WEB_APP_PORT);
            let client = build_http_client_trusting_oauth_ca(oauth_server_tls_cert_pem());
            let response = client
                .post(&url)
                .bearer_auth(token)
                .json(&CreateReadingRequest {
                    user_name: "Outage Test User".to_string(),
                    value: 99,
                    comment: None,
                })
                .send()
                .await
                .expect("POST /readings failed to send");

            assert_eq!(
                response.status().as_u16(),
                500,
                "expected 500 while calibration is in outage playbook",
            );
        }

        let recovered_id = create_reading(
            EXEC_WEB_APP_PORT,
            "Recovery Test User",
            17,
            Some("post-outage".to_string()),
        )
        .await;

        let readings = get_readings(EXEC_WEB_APP_PORT).await;
        let found = readings
            .iter()
            .find(|r| r.id == recovered_id)
            .expect("recovered reading should be present");
        assert_eq!(found.user_name, "Recovery Test User");
        assert_eq!(found.value, 17);
    });
}

#[test]
fn readings_axum_exec_readings_returns_401_when_access_token_scopes_insufficient() {
    readings_axum_component_runtime().block_on(async {
        let _arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;

        let token = fetch_example_access_token_with_scope(Some("openid profile"))
            .await;
        let url = format!("http://127.0.0.1:{}/readings", EXEC_WEB_APP_PORT);
        let client = build_http_client_trusting_oauth_ca(oauth_server_tls_cert_pem());
        let response = client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .expect("GET /readings send");

        assert_eq!(
            response.status().as_u16(),
            401,
            "GET /readings without required scope should be rejected"
        );
    });
}

#[test]
fn readings_axum_exec_readings_returns_401_when_bearer_token_invalid() {
    readings_axum_component_runtime().block_on(async {
        let _arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;

        let url = format!("http://127.0.0.1:{}/readings", EXEC_WEB_APP_PORT);
        let client = build_http_client_trusting_oauth_ca(oauth_server_tls_cert_pem());
        let response = client
            .get(&url)
            .header(
                reqwest::header::AUTHORIZATION,
                "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.signature_not_from_issuer",
            )
            .send()
            .await
            .expect("GET /readings send");

        assert_eq!(
            response.status().as_u16(),
            401,
            "GET /readings with invalid JWT should be rejected"
        );
    });
}
