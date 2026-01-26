use async_trait::async_trait;

#[async_trait]
pub trait ReadinessCheck: Send + Sync {
    /// Wait for a dependency to become ready.
    ///
    /// - `identifier`: the identifier of the dependency being checked
    /// - `target`: target address/connection info
    /// - `timeout_ms`: total time budget for the readiness check, in **milliseconds**
    async fn is_ready(
        &self,
        identifier: &str,
        target: &str,
        timeout_ms: u64,
    ) -> Result<(), String>;
}

