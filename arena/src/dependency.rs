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
    fn children(&self) -> &[Dependency];
    fn children_mut(&mut self) -> &mut [Dependency];
    async fn soft_reset(&self);
    async fn hard_reset(&mut self);
}

pub type Dependency = Box<dyn RunnableDependency>;

pub fn find_dependency<'a>(
    deps: &'a [Dependency],
    identifier: &str,
) -> Option<&'a dyn RunnableDependency> {
    for dep in deps {
        if dep.identifier() == identifier {
            return Some(dep.as_ref());
        }
        if let Some(found) = find_dependency(dep.children(), identifier) {
            return Some(found);
        }
    }
    None
}
