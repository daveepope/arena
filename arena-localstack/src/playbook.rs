use crate::localstack_dependency::resource_creator::ResourceCreator;
use crate::localstack_dependency::LocalstackDependency;

pub struct Playbook {
    endpoint: String,
    identifier: String,
    queue_urls: Vec<(String, String)>,
}

impl Playbook {
    pub fn with(dependency: &LocalstackDependency) -> Self {
        let endpoint = dependency
            .endpoint_url()
            .expect("LocalstackDependency must be started before configuring a Playbook")
            .to_string();
        Self {
            endpoint,
            identifier: format!("localstack-playbook:{}", dependency.identifier),
            queue_urls: dependency.queue_urls_snapshot(),
        }
    }

    pub fn with_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.identifier = identifier.into();
        self
    }

    pub async fn run(self) -> ActivePlaybook {
        purge_queues(&self.identifier, &self.endpoint, &self.queue_urls)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        tracing::debug!(
            playbook_id = %self.identifier,
            queue_count = self.queue_urls.len(),
            "purged queues; clean state"
        );

        ActivePlaybook {
            endpoint: self.endpoint,
            identifier: self.identifier,
            queue_urls: self.queue_urls,
        }
    }
}

fn purge_on_drop(identifier: String, endpoint: String, queue_urls: Vec<(String, String)>) {
    let already_unwinding = std::thread::panicking();

    let handle = std::thread::spawn(move || -> Result<(), String> {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::warn!(
                    playbook_id = %identifier,
                    error = %e,
                    "drop cleanup: runtime build failed"
                );
                return Ok(());
            }
        };
        rt.block_on(async move { purge_queues(&identifier, &endpoint, &queue_urls).await })
    });

    let outcome = handle.join();

    if already_unwinding {
        return;
    }

    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(msg)) => panic!("{msg}"),
        Err(_) => panic!("ActivePlaybook::drop: cleanup thread panicked"),
    }
}

pub struct ActivePlaybook {
    endpoint: String,
    identifier: String,
    queue_urls: Vec<(String, String)>,
}

impl arena::ActivePlaybook for ActivePlaybook {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ActivePlaybook {
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn queue_url(&self, name: &str) -> Option<&str> {
        self.queue_urls
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, url)| url.as_str())
    }
}

impl Drop for ActivePlaybook {
    fn drop(&mut self) {
        let identifier = std::mem::take(&mut self.identifier);
        let endpoint = std::mem::take(&mut self.endpoint);
        let queue_urls = std::mem::take(&mut self.queue_urls);

        if endpoint.is_empty() {
            return;
        }

        purge_on_drop(identifier, endpoint, queue_urls);
    }
}

async fn purge_queues(
    identifier: &str,
    endpoint: &str,
    queue_urls: &[(String, String)],
) -> Result<(), String> {
    if queue_urls.is_empty() {
        tracing::debug!(
            playbook_id = %identifier,
            "purge skipped: no queues"
        );
        return Ok(());
    }

    for (name, url) in queue_urls {
        ResourceCreator::purge_queue(endpoint, url)
            .await
            .map_err(|e| {
                format!("[LocalstackPlaybook-{identifier}] purge queue {name} failed: {e}")
            })?;
    }

    Ok(())
}
