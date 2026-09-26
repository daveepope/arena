mod arena;
mod http;
mod playbooks;
mod arena_config;

use ::arena::OpenArena;
use arena_http::{post_requested_for, ActivePlaybook as HttpActivePlaybook};
use arena_kafka::KafkaDependency;
use std::time::Duration;

use crate::arena_config::{calibration_validate_path, reset_validation_db_id};
use crate::playbooks::{calibration_api_error_path_id, calibration_api_flaky_path_id};
use crate::arena::{
    axum_component_runtime, exec_web_app_port, signed_token_with_scope,
    shared_arena, KAFKA_ID, SCENARIO_LOCK,
};
use crate::http::{
    consume_reading_created_event, create_reading, get_readings, get_readings_with_bearer_token,
    get_readings_without_token, post_reading_raw,
};

const CONSUME_READING_CREATED_EVENT_TIMEOUT_MS: u64 = 5000;

async fn kafka_bootstrap(arena: &OpenArena) -> String {
    arena
        .dependency(KAFKA_ID.get().expect("kafka id initialized"))
        .and_then(|d| d.as_any().downcast_ref::<KafkaDependency>())
        .and_then(|k| k.bootstrap_servers())
        .expect("kafka bootstrap should be available")
        .to_string()
}

async fn wait_reading_created_event<F, Fut>(bootstrap: &str, create: F) -> http::ReadingCreatedEvent
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = i32>,
{
    let (warmed_tx, warmed_rx) = tokio::sync::oneshot::channel();
    let (id_tx, id_rx) = tokio::sync::oneshot::channel();
    let bootstrap = bootstrap.to_string();
    let consume_handle = tokio::spawn(consume_reading_created_event(
        bootstrap,
        "readings".to_string(),
        warmed_tx,
        id_rx,
        Duration::from_millis(CONSUME_READING_CREATED_EVENT_TIMEOUT_MS),
    ));
    warmed_rx.await.expect("kafka consumer warmup");
    let created_id = create().await;
    id_tx
        .send(created_id as i64)
        .expect("send created_id to consumer");
    consume_handle
        .await
        .expect("consume task join")
        .expect("should consume ReadingCreatedEvent from Kafka")
}

#[test]
fn create_reading_publishes_event_and_lists_via_http() {
    axum_component_runtime().block_on(async {
        let arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;
        let bootstrap = kafka_bootstrap(arena).await;
        let _reset = arena
            .run_playbook(reset_validation_db_id())
            .await
            .expect("reset validation db playbook");

        let consumed = wait_reading_created_event(&bootstrap, || async {
            create_reading(
                exec_web_app_port(),
                "Readings API User",
                77,
                Some("kafka happy path".to_string()),
            )
            .await
        })
        .await;
        let created_id = consumed.id as i32;
        assert_eq!(consumed.id, created_id as i64);
        assert_eq!(consumed.user_name, "Readings API User");
        assert_eq!(consumed.value, 77);
        assert_eq!(consumed.comment.as_deref(), Some("kafka happy path"));

        let readings = get_readings(exec_web_app_port()).await;
        let found = readings
            .iter()
            .find(|r| r.id == created_id)
            .expect("should find newly created reading");
        assert_eq!(found.user_name, "Readings API User");
        assert_eq!(found.value, 77);
    });
}

#[test]
fn get_readings_without_bearer_token_is_rejected() {
    axum_component_runtime().block_on(async {
        let _arena = shared_arena().await;
        let status = get_readings_without_token(exec_web_app_port()).await;
        assert_eq!(
            status, 401,
            "the resource server must reject a request with no bearer token, proving it actually validates against the Cognito-shaped issuer's JWKS rather than accepting requests unconditionally"
        );
    });
}

#[test]
fn get_readings_with_token_missing_required_scope_is_rejected() {
    axum_component_runtime().block_on(async {
        let arena = shared_arena().await;
        let token = signed_token_with_scope(arena, "other-scope");
        let status = get_readings_with_bearer_token(exec_web_app_port(), &token).await;
        assert_eq!(
            status, 401,
            "a token signed by the real issuer key but missing the required 'readings' scope must be rejected, proving the resource server enforces scope and not just signature validity"
        );
    });
}

#[test]
fn create_multiple_readings_are_listed() {
    axum_component_runtime().block_on(async {
        let arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;
        let _reset = arena
            .run_playbook(reset_validation_db_id())
            .await
            .expect("reset validation db playbook");

        let id1 = create_reading(exec_web_app_port(), "Bending", 1, Some("".to_string())).await;
        let id2 = create_reading(
            exec_web_app_port(),
            "joe",
            2,
            Some("We're going to need a bigger ship".to_string()),
        )
        .await;

        let ids: Vec<i32> = get_readings(exec_web_app_port())
            .await
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    });
}

#[test]
fn post_reading_returns_500_when_calibration_outage_playbook_active() {
    axum_component_runtime().block_on(async {
        let arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;
        let _outage = arena
            .run_playbook(calibration_api_error_path_id())
            .await
            .expect("calibration error path playbook");
        let _reset = arena
            .run_playbook(reset_validation_db_id())
            .await
            .expect("reset validation db playbook");

        assert_eq!(
            post_reading_raw(exec_web_app_port(), "Outage Test User", 99, None).await,
            500,
            "expected 500 while calibration error path playbook is active"
        );
    });
}

#[test]
fn post_reading_succeeds_after_outage_playbook_scope() {
    axum_component_runtime().block_on(async {
        let arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;
        let _reset = arena
            .run_playbook(reset_validation_db_id())
            .await
            .expect("reset validation db playbook");

        let recovered_id = create_reading(
            exec_web_app_port(),
            "Recovery Test User",
            17,
            Some("post-outage".to_string()),
        )
        .await;

        let found = get_readings(exec_web_app_port())
            .await
            .into_iter()
            .find(|r| r.id == recovered_id)
            .expect("recovered reading should be present");
        assert_eq!(found.user_name, "Recovery Test User");
        assert_eq!(found.value, 17);
    });
}

#[test]
fn create_reading_with_validation_db_scoped_playbook() {
    axum_component_runtime().block_on(async {
        let arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;
        let _reset = arena
            .run_playbook(reset_validation_db_id())
            .await
            .expect("reset validation db playbook");

        let created_id = create_reading(
            exec_web_app_port(),
            "Validation DB Scoped",
            7,
            Some("mssql scope".to_string()),
        )
        .await;

        assert!(get_readings(exec_web_app_port())
            .await
            .iter()
            .any(|r| r.id == created_id));
    });
}

#[test]
fn post_reading_returns_500_under_stacked_playbooks() {
    axum_component_runtime().block_on(async {
        let arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;
        let _outage = arena
            .run_playbook(calibration_api_error_path_id())
            .await
            .expect("calibration error path playbook");
        let _reset = arena
            .run_playbook(reset_validation_db_id())
            .await
            .expect("reset validation db playbook");

        assert_eq!(
            post_reading_raw(exec_web_app_port(), "Stack Outage", 1, None).await,
            500,
            "expected 500 under stacked playbooks"
        );
    });
}

#[test]
fn post_reading_succeeds_after_calibration_flaky_sequence() {
    axum_component_runtime().block_on(async {
        let arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;
        let _flaky = arena
            .run_playbook(calibration_api_flaky_path_id())
            .await
            .expect("calibration flaky path playbook");
        let _reset = arena
            .run_playbook(reset_validation_db_id())
            .await
            .expect("reset validation db playbook");

        assert_eq!(
            post_reading_raw(exec_web_app_port(), "Flaky 1", 1, None).await,
            500,
            "first post should fail while calibration returns 500"
        );
        assert_eq!(
            post_reading_raw(exec_web_app_port(), "Flaky 2", 2, None).await,
            500,
            "second post should fail while calibration returns 503"
        );

        let created_id = create_reading(
            exec_web_app_port(),
            "Flaky 3",
            3,
            Some("recovered".to_string()),
        )
        .await;

        assert!(get_readings(exec_web_app_port())
            .await
            .iter()
            .any(|r| r.id == created_id));
    });
}

#[test]
fn http_playbook_verify_at_least_succeeds_with_traffic() {
    axum_component_runtime().block_on(async {
        let arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;
        let active = arena
            .run_playbook(calibration_api_error_path_id())
            .await
            .expect("calibration error path playbook")
            .expect("calibration error path playbook should run");
        let http = active
            .as_any()
            .downcast_ref::<HttpActivePlaybook>()
            .expect("http active playbook");

        assert_eq!(
            post_reading_raw(exec_web_app_port(), "Verify At Least", 3, None).await,
            500,
            "post should hit calibration stub"
        );

        http.verify_at_least(1, post_requested_for(calibration_validate_path()))
            .await;
    });
}

#[test]
#[should_panic(expected = "Playbook verification failed")]
fn http_playbook_verify_count_mismatch_raises() {
    axum_component_runtime().block_on(async {
        let arena = shared_arena().await;
        let _scenario = SCENARIO_LOCK.lock().await;
        let active = arena
            .run_playbook(calibration_api_error_path_id())
            .await
            .expect("calibration error path playbook")
            .expect("calibration error path playbook should run");
        let http = active
            .as_any()
            .downcast_ref::<HttpActivePlaybook>()
            .expect("http active playbook");

        http.verify(1, post_requested_for(calibration_validate_path()))
            .await;
    });
}
