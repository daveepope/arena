use async_trait::async_trait;
use tracing::Instrument;

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

    fn attach_cpu_profiler(&mut self, _limit_bytes: Option<usize>) -> Result<(), Fault> {
        Err(Fault::component(
            self.identifier().to_string(),
            "this component type does not support cpu profiling",
        ))
    }
}

pub type Component = Box<dyn RunnableComponent>;

pub async fn start_child(child: &mut Component) -> Result<(), Fault> {
    let span = crate::matches::component_span(child.identifier());
    child.start().instrument(span).await
}

pub async fn stop_child(child: &mut Component) -> Result<(), Fault> {
    let span = crate::matches::component_span(child.identifier());
    child.stop().instrument(span).await
}

pub async fn force_stop_child(child: &mut Component) {
    let span = crate::matches::component_span(child.identifier());
    child.force_stop().instrument(span).await
}

pub fn release_child(child: &mut Component) {
    let span = crate::matches::component_span(child.identifier());
    let _entered = span.enter();
    child.release();
}

pub fn component_state(comp: &dyn RunnableComponent) -> ComponentState {
    ComponentState::new(
        comp.identifier(),
        comp.state(),
        comp.faults().to_vec(),
        comp.children().iter().map(|c| component_state(c.as_ref())).collect(),
    )
}
