use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;

use crate::dependency::Dependency;
use crate::lifecycle::{Fault, RunnableState};

pub mod lifecycle;

pub use lifecycle::{ScopedPlaybookEvent, ScopedPlaybookLifecycle};

#[async_trait]
pub trait Playbook: Send + Sync {
    fn identifier(&self) -> &str;

    fn lifecycle(&self) -> &Arc<ScopedPlaybookLifecycle>;

    async fn run(&self, dependencies: &[Dependency]) -> Result<Box<dyn ActivePlaybook>, Fault>;

    fn state(&self) -> RunnableState {
        self.lifecycle().state()
    }

    fn faults(&self) -> Vec<Fault> {
        self.lifecycle().faults()
    }
}

pub trait ActivePlaybook: Send + Sync {
    fn identifier(&self) -> &str;

    fn as_any(&self) -> &dyn Any;

    fn lifecycle(&self) -> &Arc<ScopedPlaybookLifecycle>;

    fn state(&self) -> RunnableState {
        self.lifecycle().state()
    }

    fn faults(&self) -> Vec<Fault> {
        self.lifecycle().faults()
    }
}
