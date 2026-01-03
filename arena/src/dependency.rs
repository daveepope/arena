use async_trait::async_trait;

#[async_trait]
pub trait RunnableDependency: Send + Sync {
    async fn start(&mut self);
    async fn stop(&mut self);
    fn add_child(&mut self, dep: Box<dyn RunnableDependency>);
}

pub type Dependency = Box<dyn RunnableDependency>;