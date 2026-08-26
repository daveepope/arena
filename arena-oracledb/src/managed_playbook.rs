use arena::dependency::{find_dependency, Dependency};
use arena::playbook::{ActivePlaybook, Playbook as PlaybookTrait};
use async_trait::async_trait;

use crate::oracle_dependency::OracleDependency;

pub struct ManagedOraclePlaybook {
    identifier: String,
    dependency_identifier: String,
}

impl ManagedOraclePlaybook {
    pub fn new(identifier: impl Into<String>, dependency_identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
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

    async fn run(&self, dependencies: &[Dependency]) -> Box<dyn ActivePlaybook> {
        let oracle = find_dependency(dependencies, &self.dependency_identifier)
            .and_then(|d| d.as_any().downcast_ref::<OracleDependency>())
            .unwrap_or_else(|| {
                panic!(
                    "ManagedOraclePlaybook '{}': dependency '{}' not found or is not an OracleDependency",
                    self.identifier, self.dependency_identifier
                )
            });

        let playbook = oracle.playbook().with_identifier(&self.identifier);
        Box::new(playbook.run().await)
    }
}
