use arena::dependency::RunnableDependency;
use arena_temporal::TemporalDependency;
use futures::FutureExt;
use std::time::Duration;
use tokio::net::TcpStream;

fn init_test_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
}

struct TestContext {
    temporal: TemporalDependency,
    grpc_endpoint: String,
    ui_url: String,
}

impl TestContext {
    async fn new() -> Result<Self, String> {
        tracing::info!(
            suite = "crate_component",
            crate_under_test = "arena_temporal",
            phase = "dependency_start_begin",
            "starting dependency",
        );

        let mut temporal = TemporalDependency::builder("").build();

        let start_outcome = std::panic::AssertUnwindSafe(async { temporal.start().await })
            .catch_unwind()
            .await;
        match start_outcome {
            Ok(Ok(())) => {}
            Ok(Err(fault)) => {
                let _ = temporal.stop().await;
                return Err(format!("temporal failed to start: {fault}"));
            }
            Err(panic_payload) => {
                let _ = temporal.stop().await;
                std::panic::resume_unwind(panic_payload);
            }
        }

        let grpc_endpoint = temporal
            .grpc_endpoint()
            .ok_or_else(|| "temporal grpc endpoint missing after start()".to_string())?
            .to_string();

        let ui_url = temporal
            .ui_url()
            .ok_or_else(|| "temporal ui url missing after start()".to_string())?
            .to_string();

        Ok(Self {
            temporal,
            grpc_endpoint,
            ui_url,
        })
    }

    async fn stop(mut self) {
        self.temporal.stop().await.expect("temporal should stop");
    }
}

#[tokio::test]
async fn temporal_dependency_lifecycle_component_test() {
    init_test_logging();

    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    tracing::info!(
        suite = "crate_component",
        crate_under_test = "arena_temporal",
        phase = "grpc_reachability_begin",
        grpc_endpoint = %ctx.grpc_endpoint,
        "begin grpc reachability check",
    );

    let outcome = std::panic::AssertUnwindSafe(async {
        assert!(
            !ctx.ui_url.is_empty(),
            "expected a non-empty temporal ui url"
        );

        tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(&ctx.grpc_endpoint))
            .await
            .map_err(|_| format!("connect to {} timed out", ctx.grpc_endpoint))?
            .map_err(|e| format!("connect to {} failed: {e}", ctx.grpc_endpoint))?;

        Ok::<(), String>(())
    })
    .catch_unwind()
    .await;

    tokio::time::timeout(Duration::from_secs(10), ctx.stop())
        .await
        .unwrap_or_else(|_| panic!("temporal stop timed out"));

    match outcome {
        Ok(Ok(())) => tracing::info!(
            suite = "crate_component",
            crate_under_test = "arena_temporal",
            phase = "grpc_reachability_ok",
            "scenario passed",
        ),
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}
