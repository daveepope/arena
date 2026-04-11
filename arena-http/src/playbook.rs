pub mod header_pattern;
pub mod response;
pub mod stub;
pub mod verify;

use crate::http_dependency::HttpDependency;
use header_pattern::HeaderPattern;
use response::ResponseDefinition;
use stub::StubMapping;
use verify::{RequestCriteria, RecordedRequest, CountResponse, FindResponse, format_request_journal};

pub struct Playbook {
    admin_url: String,
    mappings: Vec<StubMapping>,
}

impl Playbook {
    pub fn with(dependency: &HttpDependency) -> Self {
        let admin_url = dependency
            .admin_url()
            .expect("HttpDependency must be started before configuring a Playbook");
        Self {
            admin_url,
            mappings: Vec::new(),
        }
    }

    pub fn get(self, url_path: impl Into<String>) -> PlaybookMappingBuilder {
        PlaybookMappingBuilder::new(self, "GET", url_path)
    }

    pub fn post(self, url_path: impl Into<String>) -> PlaybookMappingBuilder {
        PlaybookMappingBuilder::new(self, "POST", url_path)
    }

    pub fn put(self, url_path: impl Into<String>) -> PlaybookMappingBuilder {
        PlaybookMappingBuilder::new(self, "PUT", url_path)
    }

    pub fn delete(self, url_path: impl Into<String>) -> PlaybookMappingBuilder {
        PlaybookMappingBuilder::new(self, "DELETE", url_path)
    }

    pub async fn run(self) -> ActivePlaybook {
        let client = reqwest::Client::new();
        let mappings_url = format!("{}/mappings", self.admin_url);

        let mut registered: Vec<(String, String)> = Vec::with_capacity(self.mappings.len());

        for (idx, mapping) in self.mappings.iter().enumerate() {
            registered.push((
                mapping.method().to_string(),
                mapping.url_path().to_string(),
            ));

            let body = serde_json::to_string(mapping)
                .unwrap_or_else(|e| panic!("failed to serialize mapping {idx}: {e}"));

            let resp = client
                .post(&mappings_url)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await
                .unwrap_or_else(|e| panic!("failed to send mapping {idx}: {e}"));

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                panic!("mapping {idx} rejected by server (HTTP {status}): {body}");
            }
        }

        log::info!("Playbook applied {} mapping(s).", self.mappings.len());

        ActivePlaybook {
            admin_url: self.admin_url,
            registered,
        }
    }
}

pub struct ActivePlaybook {
    admin_url: String,
    registered: Vec<(String, String)>,
}

impl ActivePlaybook {
    fn owns(&self, criteria: &RequestCriteria) -> bool {
        let (method, path) = criteria.method_and_path();
        self.registered.iter().any(|(m, p)| {
            (method.is_none() || method.as_deref() == Some(m.as_str()))
                && (path.is_none() || path.as_deref() == Some(p.as_str()))
        })
    }

    pub async fn verify(
        &self,
        expected_count: u64,
        criteria: RequestCriteria,
    ) {
        if !self.owns(&criteria) {
            panic!(
                "\n\nPlaybook verification failed for: {criteria}\n\
                 \n  This playbook does not own a mapping for {criteria}.\n\
                 \n  Registered mappings:\n{}\n",
                self.format_registered(),
            );
        }

        let client = reqwest::Client::new();
        let body = serde_json::to_string(&criteria).expect("serialize request criteria");

        let resp = client
            .post(format!("{}/requests/count", self.admin_url))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap_or_else(|e| panic!("verify request failed: {e}"));

        let text = resp.text().await
            .unwrap_or_else(|e| panic!("verify read failed: {e}"));
        let count_resp: CountResponse = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("verify parse failed: {e}"));

        if count_resp.count != expected_count {
            let journal = self.fetch_all_requests(&client).await;
            panic!(
                "\n\nPlaybook verification failed for: {criteria}\n\
                 \n  Expected exactly {expected_count} request(s), but received {actual}.\n\
                 \nAll requests received:\n{journal}\n",
                actual = count_resp.count,
                journal = format_request_journal(&journal),
            );
        }
    }

    pub async fn verify_at_least(
        &self,
        minimum: u64,
        criteria: RequestCriteria,
    ) {
        if !self.owns(&criteria) {
            panic!(
                "\n\nPlaybook verification failed for: {criteria}\n\
                 \n  This playbook does not own a mapping for {criteria}.\n\
                 \n  Registered mappings:\n{}\n",
                self.format_registered(),
            );
        }

        let client = reqwest::Client::new();
        let body = serde_json::to_string(&criteria).expect("serialize request criteria");

        let resp = client
            .post(format!("{}/requests/count", self.admin_url))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap_or_else(|e| panic!("verify request failed: {e}"));

        let text = resp.text().await
            .unwrap_or_else(|e| panic!("verify read failed: {e}"));
        let count_resp: CountResponse = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("verify parse failed: {e}"));

        if count_resp.count < minimum {
            let journal = self.fetch_all_requests(&client).await;
            panic!(
                "\n\nPlaybook verification failed for: {criteria}\n\
                 \n  Expected at least {minimum} request(s), but received {actual}.\n\
                 \nAll requests received:\n{journal}\n",
                actual = count_resp.count,
                journal = format_request_journal(&journal),
            );
        }
    }

    pub async fn find_requests(
        &self,
        criteria: RequestCriteria,
    ) -> Vec<RecordedRequest> {
        let client = reqwest::Client::new();
        let body = serde_json::to_string(&criteria).expect("serialize request criteria");

        let resp = client
            .post(format!("{}/requests/find", self.admin_url))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap_or_else(|e| panic!("find_requests failed: {e}"));

        let text = resp.text().await
            .unwrap_or_else(|e| panic!("find_requests read failed: {e}"));
        let find_resp: FindResponse = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("find_requests parse failed: {e}"));

        find_resp.requests
    }

    async fn fetch_all_requests(
        &self,
        client: &reqwest::Client,
    ) -> Vec<RecordedRequest> {
        let resp = client
            .post(format!("{}/requests/find", self.admin_url))
            .header("Content-Type", "application/json")
            .body("{}")
            .send()
            .await;
        match resp {
            Ok(r) => {
                let text = r.text().await.unwrap_or_default();
                serde_json::from_str::<FindResponse>(&text)
                    .map(|f| f.requests)
                    .unwrap_or_default()
            }
            Err(_) => Vec::new(),
        }
    }

    fn format_registered(&self) -> String {
        if self.registered.is_empty() {
            return "  (none)".to_string();
        }
        self.registered
            .iter()
            .enumerate()
            .map(|(i, (m, p))| format!("  {}. {m} {p}", i + 1))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub struct PlaybookMappingBuilder {
    playbook: Playbook,
    mapping: stub::MappingBuilder,
}

impl PlaybookMappingBuilder {
    fn new(playbook: Playbook, method: &str, url_path: impl Into<String>) -> Self {
        Self {
            playbook,
            mapping: stub::MappingBuilder::new(method, url_path),
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, pattern: HeaderPattern) -> Self {
        self.mapping = self.mapping.with_header(name, pattern);
        self
    }

    pub fn with_request_body(mut self, body: serde_json::Value) -> Self {
        self.mapping = self.mapping.with_request_body(body);
        self
    }

    pub fn with_request_body_containing(mut self, substring: impl Into<String>) -> Self {
        self.mapping = self.mapping.with_request_body_containing(substring);
        self
    }

    pub fn in_scenario(mut self, name: impl Into<String>) -> Self {
        self.mapping = self.mapping.in_scenario(name);
        self
    }

    pub fn when_state_is(mut self, state: impl Into<String>) -> Self {
        self.mapping = self.mapping.when_state_is(state);
        self
    }

    pub fn will_set_state_to(mut self, state: impl Into<String>) -> Self {
        self.mapping = self.mapping.will_set_state_to(state);
        self
    }

    pub fn will_return(self, response: ResponseDefinition) -> PlaybookSequenceBuilder {
        PlaybookSequenceBuilder {
            playbook: self.playbook,
            mapping: self.mapping,
            responses: vec![response],
        }
    }

    pub fn will_return_in_sequence(mut self, responses: Vec<ResponseDefinition>) -> Playbook {
        assert!(!responses.is_empty(), "will_return_in_sequence requires at least one response");
        let mappings = self.mapping.will_return_sequence(responses);
        self.playbook.mappings.extend(mappings);
        self.playbook
    }
}

pub struct PlaybookSequenceBuilder {
    playbook: Playbook,
    mapping: stub::MappingBuilder,
    responses: Vec<ResponseDefinition>,
}

impl PlaybookSequenceBuilder {
    pub fn then_return(mut self, response: ResponseDefinition) -> Self {
        self.responses.push(response);
        self
    }

    fn finalize(mut self) -> Playbook {
        let mappings = self.mapping.will_return_sequence(self.responses);
        self.playbook.mappings.extend(mappings);
        self.playbook
    }

    pub fn get(self, url_path: impl Into<String>) -> PlaybookMappingBuilder {
        self.finalize().get(url_path)
    }

    pub fn post(self, url_path: impl Into<String>) -> PlaybookMappingBuilder {
        self.finalize().post(url_path)
    }

    pub fn put(self, url_path: impl Into<String>) -> PlaybookMappingBuilder {
        self.finalize().put(url_path)
    }

    pub fn delete(self, url_path: impl Into<String>) -> PlaybookMappingBuilder {
        self.finalize().delete(url_path)
    }

    pub async fn run(self) -> ActivePlaybook {
        self.finalize().run().await
    }
}
