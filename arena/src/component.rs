use async_trait::async_trait;

use crate::lifecycle::{ComponentState, Fault, RunnableState};

#[async_trait]
pub trait RunnableComponent: Send + Sync {
    fn identifier(&self) -> &str;
    fn state(&self) -> RunnableState;
    fn faults(&self) -> &[Fault];
    async fn start(&mut self) -> Result<(), Fault>;
    async fn stop(&mut self) -> Result<(), Fault>;
    async fn force_stop(&mut self);
    fn release(&mut self);
    fn add_child(&mut self, child: Box<dyn RunnableComponent>);
    fn children(&self) -> &[Component];
    fn children_mut(&mut self) -> &mut [Component];
}

pub type Component = Box<dyn RunnableComponent>;

pub fn component_state(comp: &dyn RunnableComponent) -> ComponentState {
    ComponentState::new(
        comp.identifier(),
        comp.state(),
        comp.faults().to_vec(),
        comp.children().iter().map(|c| component_state(c.as_ref())).collect(),
    )
}
