use async_trait::async_trait;
use std::any::Any;

use crate::dependency::Dependency;

#[async_trait]
pub trait Playbook: Send + Sync {
    fn identifier(&self) -> &str;

    async fn run(&self, dependencies: &[Dependency]) -> Box<dyn ActivePlaybook>;
}

pub trait ActivePlaybook: Send + Sync {
    fn identifier(&self) -> &str;

    fn as_any(&self) -> &dyn Any;
}
