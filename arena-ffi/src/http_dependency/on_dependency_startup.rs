use arena::{Dependency, OnDependencyStartup};
use arena_http::{HttpDependency, PlaybookSequenceBuilder, ResponseDefinition};
use async_trait::async_trait;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct HttpDependencyMapping {
    pub method: String,
    pub url_path: String,
    #[serde(default = "default_mapping_status")]
    pub status: u16,
    #[serde(default)]
    pub json_body: Option<serde_json::Value>,
}

fn default_mapping_status() -> u16 {
    200
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct OnDependencyStartupConfig {
    pub dependency_identifier: String,
    pub mappings: Vec<HttpDependencyMapping>,
}

pub(crate) fn build(config: OnDependencyStartupConfig) -> Box<dyn OnDependencyStartup> {
    Box::new(HttpOnDependencyStartupHandler { config })
}

struct HttpOnDependencyStartupHandler {
    config: OnDependencyStartupConfig,
}

fn response_for(m: &HttpDependencyMapping) -> ResponseDefinition {
    let mut r = ResponseDefinition::new(m.status);
    if let Some(ref body) = m.json_body {
        r = r.with_json_body(body.clone());
    }
    r
}

fn first_sequence(http: &HttpDependency, m: &HttpDependencyMapping) -> PlaybookSequenceBuilder {
    let resp = response_for(m);
    match m.method.to_ascii_uppercase().as_str() {
        "GET" => http.playbook().get(&m.url_path).will_return(resp),
        "POST" => http.playbook().post(&m.url_path).will_return(resp),
        "PUT" => http.playbook().put(&m.url_path).will_return(resp),
        "DELETE" => http.playbook().delete(&m.url_path).will_return(resp),
        other => panic!("unsupported HTTP method in on_dependency_startup mapping: {other}"),
    }
}

fn append_mapping(
    seq: PlaybookSequenceBuilder,
    m: &HttpDependencyMapping,
) -> PlaybookSequenceBuilder {
    let resp = response_for(m);
    match m.method.to_ascii_uppercase().as_str() {
        "GET" => seq.get(&m.url_path).will_return(resp),
        "POST" => seq.post(&m.url_path).will_return(resp),
        "PUT" => seq.put(&m.url_path).will_return(resp),
        "DELETE" => seq.delete(&m.url_path).will_return(resp),
        other => panic!("unsupported HTTP method in on_dependency_startup mapping: {other}"),
    }
}

#[async_trait]
impl OnDependencyStartup for HttpOnDependencyStartupHandler {
    async fn on_dependency_startup(&self, dependencies: &[Dependency]) {
        if self.config.mappings.is_empty() {
            return;
        }

        let http = dependencies
            .iter()
            .find(|d| d.identifier() == self.config.dependency_identifier)
            .and_then(|d| d.as_any().downcast_ref::<HttpDependency>())
            .unwrap_or_else(|| {
                panic!(
                    "on_dependency_startup: dependency '{}' not found or is not an HttpDependency",
                    self.config.dependency_identifier
                )
            });

        let mut seq = first_sequence(http, &self.config.mappings[0]);
        for m in self.config.mappings.iter().skip(1) {
            seq = append_mapping(seq, m);
        }
        seq.run().await.persist();
    }
}
