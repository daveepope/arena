use async_trait::async_trait;

#[async_trait]
pub trait ReadinessCheck: Send + Sync {
    async fn is_ready(&self, identifier: &str, target: &str, timeout_ms: u64)
        -> Result<(), String>;
}
