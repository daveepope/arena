use async_trait::async_trait;
use std::any::Any;

use crate::lifecycle::{DependencyState, Fault, RunnableState};

#[async_trait]
pub trait RunnableDependency: Send + Sync {
    fn identifier(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn state(&self) -> RunnableState;
    fn faults(&self) -> &[Fault];
    async fn start(&mut self) -> Result<(), Fault>;
    async fn stop(&mut self) -> Result<(), Fault>;
    async fn force_stop(&mut self);
    fn release(&mut self);
    fn add_child(&mut self, dep: Box<dyn RunnableDependency>);
    fn children(&self) -> &[Dependency];
    fn children_mut(&mut self) -> &mut [Dependency];
    async fn soft_reset(&self) -> Result<(), Fault>;
    async fn hard_reset(&mut self) -> Result<(), Fault>;
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

pub fn dependency_state(dep: &dyn RunnableDependency) -> DependencyState {
    DependencyState::new(
        dep.identifier(),
        dep.state(),
        dep.faults().to_vec(),
        dep.children().iter().map(|c| dependency_state(c.as_ref())).collect(),
    )
}
