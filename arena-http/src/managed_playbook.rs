use arena::dependency::{find_dependency, Dependency};
use arena::playbook::{ActivePlaybook, Playbook as PlaybookTrait};
use async_trait::async_trait;

use crate::http_dependency::HttpDependency;
use crate::playbook::Playbook;

type BuildFn = dyn Fn(Playbook) -> Playbook + Send + Sync;

pub struct ManagedHttpPlaybook {
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
        F: Fn(Playbook) -> Playbook + Send + Sync + 'static,
    {
        Self {
            identifier: identifier.into(),
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

    async fn run(&self, dependencies: &[Dependency]) -> Box<dyn ActivePlaybook> {
        let http = find_dependency(dependencies, &self.dependency_identifier)
            .and_then(|d| d.as_any().downcast_ref::<HttpDependency>())
            .unwrap_or_else(|| {
                panic!(
                    "ManagedHttpPlaybook '{}': dependency '{}' not found or is not an HttpDependency",
                    self.identifier, self.dependency_identifier
                )
            });

        let playbook = (self.build)(http.playbook()).with_identifier(&self.identifier);
        Box::new(playbook.run().await)
    }
}
