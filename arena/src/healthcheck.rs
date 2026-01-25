use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait ReadinessCheck: Send + Sync {
    async fn is_ready(&self, identifier: &str, target: &str, timeout: Duration)
        -> Result<(), String>;
}

