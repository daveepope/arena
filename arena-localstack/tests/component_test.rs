use std::time::{Duration, SystemTime, UNIX_EPOCH};

use arena::dependency::RunnableDependency;
use arena_localstack::LocalstackDependency;
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::Credentials;
use aws_sdk_sqs as sqs;
use futures::FutureExt;

const ACCESS_KEY: &str = "test";
const SECRET_KEY: &str = "test";
const REGION: &str = "us-east-1";

fn init_test_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
}

async fn sdk_config(endpoint: &str) -> SdkConfig {
    let creds = Credentials::new(ACCESS_KEY, SECRET_KEY, None, None, "arena-component-test");
    aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(REGION))
        .endpoint_url(endpoint)
        .credentials_provider(creds)
        .load()
        .await
}

struct TestContext {
    localstack: LocalstackDependency,
    endpoint: String,
    queue_name: String,
    queue_url: String,
}

impl TestContext {
    async fn new() -> Result<Self, String> {
        tracing::info!(
            suite = "crate_component",
            crate_under_test = "arena_localstack",
            phase = "dependency_start_begin",
            "starting dependency",
        );

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let queue_name = format!("arena-component-test-{ts}");

        let mut localstack = LocalstackDependency::builder("")
            .with_service("sqs")
            .with_queue(&queue_name)
            .build();

        localstack.start().await;

        let endpoint = match localstack.endpoint_url() {
            Some(v) => v.to_string(),
            None => {
                localstack.stop().await;
                return Err("localstack endpoint missing after start()".to_string());
            }
        };

        let queue_url = match localstack.queue_url(&queue_name) {
            Some(v) => v.to_string(),
            None => {
                localstack.stop().await;
                return Err(format!("queue url missing for {queue_name}"));
            }
        };

        tracing::info!(
            suite = "crate_component",
            crate_under_test = "arena_localstack",
            phase = "dependency_running",
            endpoint = %endpoint,
            queue_url = %queue_url,
            "dependency reachable",
        );

        Ok(Self {
            localstack,
            endpoint,
            queue_name,
            queue_url,
        })
    }

    async fn stop(mut self) {
        self.localstack.stop().await;
    }
}

async fn assert_send_receive_roundtrip(ctx: &TestContext) -> Result<(), String> {
    let config = sdk_config(&ctx.endpoint).await;
    let client = sqs::Client::new(&config);

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let body = format!("hello-from-component-test-{ts}");

    client
        .send_message()
        .queue_url(&ctx.queue_url)
        .message_body(&body)
        .send()
        .await
        .map_err(|e| format!("send_message failed: {e}"))?;

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        let resp = client
            .receive_message()
            .queue_url(&ctx.queue_url)
            .max_number_of_messages(1)
            .wait_time_seconds(1)
            .send()
            .await
            .map_err(|e| format!("receive_message failed: {e}"))?;

        if let Some(messages) = resp.messages {
            for msg in messages {
                if msg.body.as_deref() == Some(body.as_str()) {
                    return Ok(());
                }
            }
        }
    }

    Err(format!(
        "did not observe message on queue {}",
        ctx.queue_name
    ))
}

#[tokio::test]
async fn localstack_dependency_lifecycle_component_test() {
    init_test_logging();
    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_localstack",
        phase = "sqs_roundtrip_begin",
        queue = %ctx.queue_name,
        "begin send receive",
    );
    let outcome = std::panic::AssertUnwindSafe(assert_send_receive_roundtrip(&ctx))
        .catch_unwind()
        .await;

    ctx.stop().await;

    match outcome {
        Ok(Ok(())) => {
            tracing::info!(
                suite = "crate_component",
                crate_under_test = "arena_localstack",
                phase = "sqs_roundtrip_ok",
                "scenario passed",
            );
        }
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}
