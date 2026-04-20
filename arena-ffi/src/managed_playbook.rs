use arena::Playbook;
use arena_http::{
    ManagedHttpPlaybook, Playbook as HttpPlaybook, PlaybookSequenceBuilder,
    ResponseDefinition,
};
use arena_localstack::ManagedLocalstackPlaybook;
use arena_mssql::ManagedMssqlPlaybook;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ManagedPlaybookConfig {
    pub identifier: String,
    #[serde(default = "default_exec_on_dependency_start")]
    pub exec_on_dependency_start: bool,
    #[serde(flatten)]
    pub kind: PlaybookKindConfig,
}

fn default_exec_on_dependency_start() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum PlaybookKindConfig {
    Http(HttpPlaybookConfig),
    Mssql(MssqlPlaybookConfig),
    Localstack(LocalstackPlaybookConfig),
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct HttpPlaybookConfig {
    pub dependency_identifier: String,
    pub mappings: Vec<HttpMapping>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct HttpMapping {
    pub method: String,
    pub url_path: String,
    #[serde(default = "default_status")]
    pub status: u16,
    #[serde(default)]
    pub json_body: Option<serde_json::Value>,
}

fn default_status() -> u16 {
    200
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MssqlPlaybookConfig {
    pub dependency_identifier: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct LocalstackPlaybookConfig {
    pub dependency_identifier: String,
}

pub(crate) fn build(config: ManagedPlaybookConfig) -> Box<dyn Playbook> {
    match config.kind {
        PlaybookKindConfig::Http(http) => Box::new(ManagedHttpPlaybook::new(
            config.identifier,
            http.dependency_identifier,
            move |pb| build_http_playbook(pb, &http.mappings),
        )),
        PlaybookKindConfig::Mssql(mssql) => Box::new(ManagedMssqlPlaybook::new(
            config.identifier,
            mssql.dependency_identifier,
        )),
        PlaybookKindConfig::Localstack(localstack) => Box::new(ManagedLocalstackPlaybook::new(
            config.identifier,
            localstack.dependency_identifier,
        )),
    }
}

fn response_for(m: &HttpMapping) -> ResponseDefinition {
    let mut r = ResponseDefinition::new(m.status);
    if let Some(ref body) = m.json_body {
        r = r.with_json_body(body.clone());
    }
    r
}

fn build_http_playbook(pb: HttpPlaybook, mappings: &[HttpMapping]) -> HttpPlaybook {
    assert!(
        !mappings.is_empty(),
        "http playbook registration requires at least one mapping"
    );

    let mut seq = first_sequence(pb, &mappings[0]);
    for m in mappings.iter().skip(1) {
        seq = append_mapping(seq, m);
    }
    seq.into_playbook()
}

fn first_sequence(pb: HttpPlaybook, m: &HttpMapping) -> PlaybookSequenceBuilder {
    let resp = response_for(m);
    match m.method.to_ascii_uppercase().as_str() {
        "GET" => pb.get(&m.url_path).will_return(resp),
        "POST" => pb.post(&m.url_path).will_return(resp),
        "PUT" => pb.put(&m.url_path).will_return(resp),
        "DELETE" => pb.delete(&m.url_path).will_return(resp),
        other => panic!("unsupported HTTP method in playbook registration: {other}"),
    }
}

fn append_mapping(
    seq: PlaybookSequenceBuilder,
    m: &HttpMapping,
) -> PlaybookSequenceBuilder {
    let resp = response_for(m);
    match m.method.to_ascii_uppercase().as_str() {
        "GET" => seq.get(&m.url_path).will_return(resp),
        "POST" => seq.post(&m.url_path).will_return(resp),
        "PUT" => seq.put(&m.url_path).will_return(resp),
        "DELETE" => seq.delete(&m.url_path).will_return(resp),
        other => panic!("unsupported HTTP method in playbook registration: {other}"),
    }
}
