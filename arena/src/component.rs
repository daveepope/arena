use async_trait::async_trait;

#[async_trait]
pub trait RunnableComponent: Send + Sync {
    async fn start(&mut self);
    async fn stop(&mut self);
    fn add_child(&mut self, child: Box<dyn RunnableComponent>);
}

pub type Component = Box<dyn RunnableComponent>;
