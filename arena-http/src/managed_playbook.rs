use arena::dependency::{find_dependency, Dependency};
use arena::lifecycle::{Fault, RunnableState};
use arena::playbook::{ActivePlaybook, Playbook as PlaybookTrait, ScopedPlaybookLifecycle};
use std::sync::Arc;
use async_trait::async_trait;

use crate::http_dependency::HttpDependency;
use crate::playbook::Playbook;

type BuildFn = dyn Fn(Playbook) -> Result<Playbook, Fault> + Send + Sync;

pub struct ManagedHttpPlaybook {
    lifecycle: Arc<ScopedPlaybookLifecycle>,
    identifier: String,
    dependency_identifier: String,
    build: Box<BuildFn>,
}

impl ManagedHttpPlaybook {
    pub fn new<F>(
        identifier: impl Into<String>,
        dependency_identifier: impl Into<String>,
        build: F,
    ) -> Self
    where
        F: Fn(Playbook) -> Result<Playbook, Fault> + Send + Sync + 'static,
    {
        let identifier = identifier.into();
        Self {
            lifecycle: ScopedPlaybookLifecycle::new(identifier.clone()),
            identifier,
            dependency_identifier: dependency_identifier.into(),
            build: Box::new(build),
        }
    }

    pub fn into_box(self) -> Box<dyn PlaybookTrait> {
        Box::new(self)
    }
}

#[async_trait]
impl PlaybookTrait for ManagedHttpPlaybook {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn lifecycle(&self) -> &Arc<ScopedPlaybookLifecycle> {
        &self.lifecycle
    }

    async fn run(&self, dependencies: &[Dependency]) -> Result<Box<dyn ActivePlaybook>, Fault> {
        self.lifecycle.report(RunnableState::Starting);
        match self.run_inner(dependencies).await {
            Ok(active) => {
                self.lifecycle.report(RunnableState::Started);
                Ok(active)
            }
            Err(fault) => Err(fault),
        }
    }
}

impl ManagedHttpPlaybook {
    async fn run_inner(
        &self,
        dependencies: &[Dependency],
    ) -> Result<Box<dyn ActivePlaybook>, Fault> {
        let http = find_dependency(dependencies, &self.dependency_identifier)
            .and_then(|d| d.as_any().downcast_ref::<HttpDependency>())
            .ok_or_else(|| {
                Fault::playbook(
                    &self.identifier,
                    format!(
                        "dependency '{}' not found or is not an HttpDependency",
                        self.dependency_identifier
                    ),
                )
            })?;

        if http.admin_url().is_none() {
            return Err(Fault::playbook(
                &self.identifier,
                format!(
                    "dependency '{}' is not started",
                    self.dependency_identifier
                ),
            ));
        }

        let playbook = (self.build)(http.playbook())?
            .with_identifier(&self.identifier)
            .with_lifecycle(Arc::clone(&self.lifecycle));
        let active = playbook.run().await;
        self.lifecycle.report(RunnableState::Started);
        Ok(Box::new(active))
    }
}
