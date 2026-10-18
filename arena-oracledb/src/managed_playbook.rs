use arena::dependency::{find_dependency, Dependency};
use arena::lifecycle::Fault;
use arena::playbook::{
    ActivePlaybook, ManagedPlaybook, Playbook as PlaybookTrait, ScopedPlaybookLifecycle,
};
use std::sync::Arc;
use async_trait::async_trait;

use crate::oracle_dependency::OracleDependency;

pub struct ManagedOraclePlaybook {
    managed: ManagedPlaybook,
    identifier: String,
    dependency_identifier: String,
}

impl ManagedOraclePlaybook {
    pub fn new(identifier: impl Into<String>, dependency_identifier: impl Into<String>) -> Self {
        let identifier = identifier.into();
        Self {
            managed: ManagedPlaybook::new(identifier.clone()),
            identifier,
            dependency_identifier: dependency_identifier.into(),
        }
    }

    pub fn into_box(self) -> Box<dyn PlaybookTrait> {
        Box::new(self)
    }
}

#[async_trait]
impl PlaybookTrait for ManagedOraclePlaybook {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn lifecycle(&self) -> &Arc<ScopedPlaybookLifecycle> {
        self.managed.lifecycle()
    }

    async fn run(&self, dependencies: &[Dependency]) -> Result<Box<dyn ActivePlaybook>, Fault> {
        self.managed.apply(self.run_inner(dependencies)).await
    }
}

impl ManagedOraclePlaybook {
    async fn run_inner(
        &self,
        dependencies: &[Dependency],
    ) -> Result<Box<dyn ActivePlaybook>, Fault> {
        let oracle = find_dependency(dependencies, &self.dependency_identifier)
            .and_then(|d| d.as_any().downcast_ref::<OracleDependency>())
            .ok_or_else(|| {
                Fault::playbook(
                    &self.identifier,
                    format!(
                        "dependency '{}' not found or is not an OracleDependency",
                        self.dependency_identifier
                    ),
                )
            })?;

        if oracle.connection_string().is_none() {
            return Err(Fault::playbook(
                &self.identifier,
                format!(
                    "dependency '{}' is not started",
                    self.dependency_identifier
                ),
            ));
        }

        let playbook = oracle.playbook().with_identifier(&self.identifier)
            .with_lifecycle(Arc::clone(self.managed.lifecycle()));
        Ok(Box::new(playbook.run().await))
    }
}
