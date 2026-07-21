use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use futures_timer::Delay;
use std::time::{Duration, Instant};
use tonic::transport::{Channel, Endpoint};
use tonic::Code;
use tonic_health::pb::health_client::HealthClient;
use tonic_health::pb::HealthCheckRequest;

pub(super) struct DefaultTemporalReadinessCheck;

#[async_trait]
impl ReadinessCheck for DefaultTemporalReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        grpc_endpoint: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let timeout = Duration::from_millis(timeout_ms);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();
        let endpoint = format!("http://{grpc_endpoint}");

        loop {
            match probe_once(&endpoint).await {
                Ok(()) => return Ok(()),
                Err(err) => {
                    if start.elapsed() >= timeout {
                        return Err(format!(
                            "[Temporal-{identifier}] readiness probe against {grpc_endpoint} \
                             did not succeed within {timeout:?}: {err}"
                        ));
                    }

                    tracing::debug!(
                        subsystem = "temporal",
                        error = %err,
                        "readiness probe failed (will retry)"
                    );
                    Delay::new(poll_every).await;
                }
            }
        }
    }
}

async fn probe_once(endpoint: &str) -> Result<(), String> {
    let channel: Channel = Endpoint::from_shared(endpoint.to_string())
        .map_err(|err| err.to_string())?
        .connect()
        .await
        .map_err(|err| err.to_string())?;
    let mut client = HealthClient::new(channel);

    match client
        .check(HealthCheckRequest {
            service: String::new(),
        })
        .await
    {
        Ok(_) => Ok(()),
        Err(status) if status.code() != Code::Unavailable => Ok(()),
        Err(status) => Err(status.to_string()),
    }
}
