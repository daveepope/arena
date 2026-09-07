use std::time::{Duration, SystemTime, UNIX_EPOCH};

use arena::dependency::RunnableDependency;
use arena_localstack::{
    EventRuleSpec, EventRuleTarget, EventTargetKind, LocalstackDependency, ResourceCreator,
};
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::Credentials;
use aws_sdk_sqs as sqs;
use aws_smithy_http_client::tls::rustls_provider::CryptoMode;
use aws_smithy_http_client::{tls, Builder as HttpClientBuilder};
use futures::FutureExt;

const EPHEMERAL_PORT_RANGE: std::ops::RangeInclusive<u16> = 21350..=21399;

fn ephemeral_tcp_port() -> u16 {
    arena_host::find_available_port::find_available_port(
        EPHEMERAL_PORT_RANGE,
        arena_host::find_available_port::PortSearchStrategy::Random,
    )
    .unwrap_or_else(|| {
        panic!(
            "no available port found in range {}..={}",
            EPHEMERAL_PORT_RANGE.start(), EPHEMERAL_PORT_RANGE.end()
        )
    })
}

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
    let http_client = HttpClientBuilder::new()
        .tls_provider(tls::Provider::Rustls(CryptoMode::Ring))
        .build_https();
    aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(REGION))
        .endpoint_url(endpoint)
        .credentials_provider(creds)
        .http_client(http_client)
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
        let fifo_queue_name = format!("arena-component-test-{ts}.fifo");
        let event_bus_name = format!("arena-component-test-bus-{ts}");
        let event_rule_name = format!("arena-component-test-rule-{ts}");

        let mut localstack = LocalstackDependency::builder("localstack-component")
            .with_port(ephemeral_tcp_port())
            .with_service("sqs")
            .with_service("events")
            .with_queue(&queue_name)
            .with_fifo_queue(&fifo_queue_name)
            .with_event_bus(&event_bus_name)
            .with_event_rule(EventRuleSpec {
                name: event_rule_name,
                event_bus: Some(event_bus_name.clone()),
                event_pattern: r#"{"source": ["arena.component-test"]}"#.to_string(),
                targets: vec![EventRuleTarget {
                    target_id: "queue-target".to_string(),
                    kind: EventTargetKind::SqsQueue {
                        queue_name: queue_name.clone(),
                    },
                }],
            })
            .build();

        let start_outcome = std::panic::AssertUnwindSafe(async { localstack.start().await })
            .catch_unwind()
            .await;
        if let Err(panic_payload) = start_outcome {
            localstack.stop().await.expect("stop should succeed");
            std::panic::resume_unwind(panic_payload);
        }

        let endpoint = match localstack.endpoint_url() {
            Some(v) => v.to_string(),
            None => {
                localstack.stop().await.expect("stop should succeed");
                return Err("localstack endpoint missing after start()".to_string());
            }
        };

        let queue_url = match localstack.queue_url(&queue_name) {
            Some(v) => v.to_string(),
            None => {
                localstack.stop().await.expect("stop should succeed");
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
        self.localstack.stop().await.expect("stop should succeed");
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

async fn assert_purge_queue_succeeds(ctx: &TestContext) -> Result<(), String> {
    ResourceCreator::purge_queue(&ctx.endpoint, &ctx.queue_url).await
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
    let scenario = async {
        assert_send_receive_roundtrip(&ctx).await?;
        assert_purge_queue_succeeds(&ctx).await
    };
    let outcome = std::panic::AssertUnwindSafe(scenario).catch_unwind().await;

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
