use async_trait::async_trait;
use std::any::Any;

#[async_trait]
pub trait RunnableDependency: Send + Sync {
    fn identifier(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    async fn start(&mut self);
    async fn stop(&mut self);
    fn add_child(&mut self, dep: Box<dyn RunnableDependency>);
    async fn soft_reset(&self);
    async fn hard_reset(&mut self);
}

pub type Dependency = Box<dyn RunnableDependency>;