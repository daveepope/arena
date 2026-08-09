use arena::dependency::{find_dependency, Dependency};
use arena::playbook::{ActivePlaybook, Playbook as PlaybookTrait};
use async_trait::async_trait;

use crate::mssql_dependency::MssqlDependency;

pub struct ManagedMssqlPlaybook {
    identifier: String,
    dependency_identifier: String,
}

impl ManagedMssqlPlaybook {
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
impl PlaybookTrait for ManagedMssqlPlaybook {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    async fn run(&self, dependencies: &[Dependency]) -> Box<dyn ActivePlaybook> {
        let mssql = find_dependency(dependencies, &self.dependency_identifier)
            .and_then(|d| d.as_any().downcast_ref::<MssqlDependency>())
            .unwrap_or_else(|| {
                panic!(
                    "ManagedMssqlPlaybook '{}': dependency '{}' not found or is not an MssqlDependency",
                    self.identifier, self.dependency_identifier
                )
            });

        let playbook = mssql.playbook().with_identifier(&self.identifier);
        Box::new(playbook.run().await)
    }
}
