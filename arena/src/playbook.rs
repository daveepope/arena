use async_trait::async_trait;
use std::any::Any;
use std::future::Future;
use std::sync::Arc;

use crate::dependency::Dependency;
use crate::lifecycle::Fault;

pub mod lifecycle;
pub mod state;

pub use lifecycle::{PlaybookKind, PlaybookType, PlaybookEvent, PlaybookLifecycle};
pub use state::{PlaybookRule, PlaybookState};

pub struct ManagedPlaybook {
    lifecycle: Arc<PlaybookLifecycle>,
}

impl ManagedPlaybook {
    pub fn new(identifier: impl Into<String>) -> Self {
        Self {
            lifecycle: PlaybookLifecycle::managed(identifier),
        }
    }

    pub fn lifecycle(&self) -> &Arc<PlaybookLifecycle> {
        &self.lifecycle
    }

    pub async fn apply<F>(&self, apply: F) -> Result<Box<dyn ActivePlaybook>, Fault>
    where
        F: Future<Output = Result<Box<dyn ActivePlaybook>, Fault>>,
    {
        report_around(&self.lifecycle, apply).await
    }
}

pub struct UnmanagedPlaybook {
    lifecycle: Arc<PlaybookLifecycle>,
}

impl UnmanagedPlaybook {
    pub fn new(identifier: impl Into<String>) -> Self {
        Self {
            lifecycle: PlaybookLifecycle::new(identifier),
        }
    }

    pub fn lifecycle(&self) -> &Arc<PlaybookLifecycle> {
        &self.lifecycle
    }

    pub async fn apply<F>(&self, apply: F) -> Result<Box<dyn ActivePlaybook>, Fault>
    where
        F: Future<Output = Result<Box<dyn ActivePlaybook>, Fault>>,
    {
        report_around(&self.lifecycle, apply).await
    }
}

async fn report_around<F>(
    lifecycle: &PlaybookLifecycle,
    apply: F,
) -> Result<Box<dyn ActivePlaybook>, Fault>
where
    F: Future<Output = Result<Box<dyn ActivePlaybook>, Fault>>,
{
    lifecycle.report_running();
    match apply.await {
        Ok(active) => {
            lifecycle.enter();
            Ok(active)
        }
        Err(fault) => {
            lifecycle.report_fault(fault.clone());
            Err(fault)
        }
    }
}

#[async_trait]
pub trait Playbook: Send + Sync {
    fn identifier(&self) -> &str;

    fn lifecycle(&self) -> &Arc<PlaybookLifecycle>;

    async fn run(&self, dependencies: &[Dependency]) -> Result<Box<dyn ActivePlaybook>, Fault>;

    fn state(&self) -> PlaybookState {
        self.lifecycle().state()
    }

    fn faults(&self) -> Vec<Fault> {
        self.lifecycle().faults()
    }
}

pub trait ActivePlaybook: Send + Sync {
    fn identifier(&self) -> &str;

    fn as_any(&self) -> &dyn Any;

    fn lifecycle(&self) -> &Arc<PlaybookLifecycle>;

    fn state(&self) -> PlaybookState {
        self.lifecycle().state()
    }

    fn faults(&self) -> Vec<Fault> {
        self.lifecycle().faults()
    }
}
