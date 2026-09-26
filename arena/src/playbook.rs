use async_trait::async_trait;
use std::any::Any;

use crate::dependency::Dependency;
use crate::lifecycle::Fault;

#[async_trait]
pub trait Playbook: Send + Sync {
    fn identifier(&self) -> &str;

    async fn run(&self, dependencies: &[Dependency]) -> Result<Box<dyn ActivePlaybook>, Fault>;
}

pub trait ActivePlaybook: Send + Sync {
    fn identifier(&self) -> &str;

    fn as_any(&self) -> &dyn Any;
}
