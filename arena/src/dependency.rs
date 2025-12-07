pub trait RunnableDependency: Send + Sync {
    fn start(&mut self);
    fn stop(&mut self);
    fn add_child(&mut self, dep: Box<dyn RunnableDependency>);
}

pub type Dependency = Box<dyn RunnableDependency>;