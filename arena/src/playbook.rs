use async_trait::async_trait;
use std::any::Any;
use std::future::Future;
use std::sync::Arc;

use crate::dependency::Dependency;
use crate::lifecycle::{Fault, RunnableState};

pub mod lifecycle;

pub use lifecycle::{PlaybookKind, PlaybookType, ScopedPlaybookEvent, ScopedPlaybookLifecycle};

pub struct ManagedPlaybook {
    lifecycle: Arc<ScopedPlaybookLifecycle>,
}

impl ManagedPlaybook {
    pub fn new(identifier: impl Into<String>) -> Self {
        Self {
            lifecycle: ScopedPlaybookLifecycle::managed(identifier),
        }
    }

    pub fn lifecycle(&self) -> &Arc<ScopedPlaybookLifecycle> {
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
    lifecycle: Arc<ScopedPlaybookLifecycle>,
}

impl UnmanagedPlaybook {
    pub fn new(identifier: impl Into<String>) -> Self {
        Self {
            lifecycle: ScopedPlaybookLifecycle::new(identifier),
        }
    }

    pub fn lifecycle(&self) -> &Arc<ScopedPlaybookLifecycle> {
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
    lifecycle: &ScopedPlaybookLifecycle,
    apply: F,
) -> Result<Box<dyn ActivePlaybook>, Fault>
where
    F: Future<Output = Result<Box<dyn ActivePlaybook>, Fault>>,
{
    lifecycle.report_starting();
    let outcome = apply.await;
    if outcome.is_ok() {
        lifecycle.report_started();
    }
    outcome
}

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
