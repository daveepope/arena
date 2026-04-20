pub mod header_pattern;
pub mod response;
pub mod stub;
pub mod verify;

use crate::http_dependency::HttpDependency;
use header_pattern::HeaderPattern;
use response::ResponseDefinition;
use serde::Deserialize;
use std::collections::HashSet;
use stub::StubMapping;
use verify::{
    RequestCriteria, RecordedRequest, FindResponse,
    ServeEvent, ServeEventsResponse, event_matches_criteria, event_stub_id,
    format_request_journal,
};

#[derive(Debug, Clone)]
struct RegisteredMapping {
    id: String,
    method: String,
    url_path: String,
}

#[derive(Deserialize)]
struct MappingCreated {
    id: String,
}

#[derive(Debug, Clone, Copy)]
pub enum ExpectedCount {
    Exactly(u64),
    AtLeast(u64),
    Never,
}

#[derive(Debug, Clone)]
struct Expectation {
    method: String,
    url_path: String,
    count: ExpectedCount,
}

impl Expectation {
    fn describe(&self) -> String {
        match self.count {
            ExpectedCount::Exactly(n) => format!("{} {} called exactly {n} time(s)", self.method, self.url_path),
            ExpectedCount::AtLeast(n) => format!("{} {} called at least {n} time(s)", self.method, self.url_path),
            ExpectedCount::Never => format!("{} {} never called", self.method, self.url_path),
        }
    }

    fn check(&self, actual: u64) -> Result<(), String> {
        match self.count {
            ExpectedCount::Exactly(n) if actual != n => Err(format!(
                "  - expected {} {} to be called exactly {n} time(s), but saw {actual}",
                self.method, self.url_path
            )),
            ExpectedCount::AtLeast(n) if actual < n => Err(format!(
                "  - expected {} {} to be called at least {n} time(s), but saw {actual}",
                self.method, self.url_path
            )),
            ExpectedCount::Never if actual != 0 => Err(format!(
                "  - expected {} {} to never be called, but saw {actual} call(s)",
                self.method, self.url_path
            )),
            _ => Ok(()),
        }
    }
}

pub struct Playbook {
    identifier: String,
    admin_url: String,
    mappings: Vec<StubMapping>,
    expectations: Vec<Expectation>,
}

impl Playbook {
    pub fn with(dependency: &HttpDependency) -> Self {
        let admin_url = dependency
            .admin_url()
            .expect("HttpDependency must be started before configuring a Playbook");
        Self {
            identifier: format!("http-playbook:{}", dependency.identifier),
            admin_url,
            mappings: Vec::new(),
            expectations: Vec::new(),
        }
    }

    pub fn with_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.identifier = identifier.into();
        self
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

        let mut registered: Vec<RegisteredMapping> = Vec::with_capacity(self.mappings.len());

        for (idx, mapping) in self.mappings.iter().enumerate() {
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

            let text = resp.text().await
                .unwrap_or_else(|e| panic!("read mapping {idx} response: {e}"));
            let created: MappingCreated = serde_json::from_str(&text)
                .unwrap_or_else(|e| panic!("parse mapping {idx} response: {e}\nbody: {text}"));

            registered.push(RegisteredMapping {
                id: created.id,
                method: mapping.method().to_string(),
                url_path: mapping.url_path().to_string(),
            });
        }

        log::info!(
            "Playbook applied {} mapping(s), {} expectation(s).",
            self.mappings.len(),
            self.expectations.len(),
        );

        ActivePlaybook {
            identifier: self.identifier,
            admin_url: self.admin_url,
            registered,
            expectations: self.expectations,
        }
    }
}

pub struct ActivePlaybook {
    identifier: String,
    admin_url: String,
    registered: Vec<RegisteredMapping>,
    expectations: Vec<Expectation>,
}

impl arena::ActivePlaybook for ActivePlaybook {
    fn identifier(&self) -> &str {
        &self.identifier
    }
}

impl ActivePlaybook {
    pub fn admin_url(&self) -> &str {
        &self.admin_url
    }

    pub fn persist(mut self) {
        self.registered.clear();
        self.expectations.clear();
    }

    fn owns(&self, criteria: &RequestCriteria) -> bool {
        let (method, path) = criteria.method_and_path();
        self.registered.iter().any(|r| {
            (method.is_none() || method.as_deref() == Some(r.method.as_str()))
                && (path.is_none() || path.as_deref() == Some(r.url_path.as_str()))
        })
    }

    fn owned_ids(&self) -> HashSet<&str> {
        self.registered.iter().map(|r| r.id.as_str()).collect()
    }

    pub async fn verify(&self, expected_count: u64, criteria: RequestCriteria) {
        let actual = self.scoped_count(&criteria).await;
        if actual != expected_count {
            let client = reqwest::Client::new();
            let all = self.fetch_all_requests(&client).await;
            panic!(
                "\n\nPlaybook verification failed for: {criteria}\n\
                 \n  Expected exactly {expected_count} request(s), but received {actual}.\n\
                 \nAll requests received:\n{journal}\n",
                journal = format_request_journal(&all),
            );
        }
    }

    pub async fn verify_at_least(&self, minimum: u64, criteria: RequestCriteria) {
        let actual = self.scoped_count(&criteria).await;
        if actual < minimum {
            let client = reqwest::Client::new();
            let all = self.fetch_all_requests(&client).await;
            panic!(
                "\n\nPlaybook verification failed for: {criteria}\n\
                 \n  Expected at least {minimum} request(s), but received {actual}.\n\
                 \nAll requests received:\n{journal}\n",
                journal = format_request_journal(&all),
            );
        }
    }

    pub async fn find_requests(&self, criteria: RequestCriteria) -> Vec<RecordedRequest> {
        if criteria.has_header_criteria() {
            panic!(
                "ActivePlaybook::find_requests does not support header criteria in scope-local \
                 mode (criteria: {criteria}). Match on method + urlPath only."
            );
        }
        if !self.owns(&criteria) {
            self.panic_not_owned(&criteria);
        }

        let client = reqwest::Client::new();
        let events = self.fetch_scope_events(&client).await;
        let matched: Vec<&ServeEvent> = events
            .iter()
            .filter(|ev| event_matches_criteria(ev, &criteria))
            .collect();

        self.hydrate_recorded(&client, &matched).await
    }

    async fn scoped_count(&self, criteria: &RequestCriteria) -> u64 {
        if criteria.has_header_criteria() {
            panic!(
                "ActivePlaybook::verify does not support header criteria in scope-local mode \
                 (criteria: {criteria}). Match on method + urlPath only."
            );
        }
        if !self.owns(criteria) {
            self.panic_not_owned(criteria);
        }

        let client = reqwest::Client::new();
        let events = self.fetch_scope_events(&client).await;
        events
            .iter()
            .filter(|ev| event_matches_criteria(ev, criteria))
            .count() as u64
    }

    async fn fetch_scope_events(&self, client: &reqwest::Client) -> Vec<ServeEvent> {
        fetch_scope_events_for(client, &self.admin_url, &self.owned_ids()).await
    }

    async fn hydrate_recorded(
        &self,
        client: &reqwest::Client,
        events: &[&ServeEvent],
    ) -> Vec<RecordedRequest> {
        if events.is_empty() {
            return Vec::new();
        }

        let resp = client
            .post(format!("{}/requests/find", self.admin_url))
            .header("Content-Type", "application/json")
            .body("{}")
            .send()
            .await;
        let all: Vec<RecordedRequest> = match resp {
            Ok(r) => {
                let text = r.text().await.unwrap_or_default();
                serde_json::from_str::<FindResponse>(&text)
                    .map(|f| f.requests)
                    .unwrap_or_default()
            }
            Err(_) => Vec::new(),
        };

        let mut out: Vec<RecordedRequest> = Vec::with_capacity(events.len());
        for ev in events {
            let logged_path = ev.request.url.split('?').next().unwrap_or(&ev.request.url);
            if let Some(found) = all.iter().find(|r: &&RecordedRequest| {
                r.method.eq_ignore_ascii_case(&ev.request.method)
                    && r.url.split('?').next().unwrap_or(&r.url) == logged_path
            }) {
                out.push(found.clone());
            }
        }
        out
    }

    async fn fetch_all_requests(&self, client: &reqwest::Client) -> Vec<RecordedRequest> {
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

    fn panic_not_owned(&self, criteria: &RequestCriteria) -> ! {
        panic!(
            "\n\nPlaybook verification failed for: {criteria}\n\
             \n  This playbook does not own a mapping for {criteria}.\n\
             \n  Registered mappings:\n{}\n",
            self.format_registered(),
        );
    }

    fn format_registered(&self) -> String {
        if self.registered.is_empty() {
            return "  (none)".to_string();
        }
        self.registered
            .iter()
            .enumerate()
            .map(|(i, r)| format!("  {}. {} {}", i + 1, r.method, r.url_path))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

async fn fetch_scope_events_for(
    client: &reqwest::Client,
    admin_url: &str,
    owned: &HashSet<&str>,
) -> Vec<ServeEvent> {
    let resp = client
        .get(format!("{admin_url}/requests?limit=1000"))
        .send()
        .await
        .unwrap_or_else(|e| panic!("fetch serve events failed: {e}"));

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        panic!("fetch serve events got HTTP {status}: {text}");
    }

    let text = resp.text().await
        .unwrap_or_else(|e| panic!("read serve events failed: {e}"));
    let parsed: ServeEventsResponse = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("parse serve events failed: {e}\nbody: {text}"));

    parsed
        .requests
        .into_iter()
        .filter(|ev| event_stub_id(ev).map(|id| owned.contains(id)).unwrap_or(false))
        .collect()
}

impl Drop for ActivePlaybook {
    fn drop(&mut self) {
        let already_unwinding = std::thread::panicking();
        let expectations = std::mem::take(&mut self.expectations);
        let registered = std::mem::take(&mut self.registered);

        if registered.is_empty() && expectations.is_empty() {
            return;
        }

        let admin_url = self.admin_url.clone();
        let mapping_ids: Vec<String> = registered.iter().map(|r| r.id.clone()).collect();

        let handle = std::thread::spawn(move || -> Result<(), String> {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    log::warn!("ActivePlaybook::drop: failed to build runtime: {e}");
                    return Ok(());
                }
            };
            rt.block_on(async move {
                let client = reqwest::Client::new();

                let verify_result = if already_unwinding || expectations.is_empty() {
                    Ok(())
                } else {
                    verify_expectations(&client, &admin_url, &mapping_ids, &expectations).await
                };

                delete_owned_journal_entries(&client, &admin_url, &mapping_ids).await;

                for id in &mapping_ids {
                    let url = format!("{admin_url}/mappings/{id}");
                    match client.delete(&url).send().await {
                        Ok(r) if r.status().is_success() => {
                            log::debug!("ActivePlaybook::drop: deleted mapping {id}");
                        }
                        Ok(r) => log::warn!(
                            "ActivePlaybook::drop: delete mapping {id} got HTTP {}",
                            r.status()
                        ),
                        Err(e) => log::warn!(
                            "ActivePlaybook::drop: delete mapping {id} failed: {e}"
                        ),
                    }
                }

                verify_result
            })
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
}

async fn verify_expectations(
    client: &reqwest::Client,
    admin_url: &str,
    mapping_ids: &[String],
    expectations: &[Expectation],
) -> Result<(), String> {
    let owned: HashSet<&str> = mapping_ids.iter().map(|s| s.as_str()).collect();
    let events = fetch_scope_events_for(client, admin_url, &owned).await;

    let mut errors: Vec<String> = Vec::new();

    for expect in expectations {
        let actual = events
            .iter()
            .filter(|ev| {
                ev.request.method.eq_ignore_ascii_case(&expect.method)
                    && {
                        let p = ev.request.url.split('?').next().unwrap_or(&ev.request.url);
                        p == expect.url_path
                    }
            })
            .count() as u64;

        if let Err(msg) = expect.check(actual) {
            errors.push(msg);
        }
    }

    if errors.is_empty() {
        return Ok(());
    }

    let expectations_desc = expectations
        .iter()
        .map(|e| format!("  - {}", e.describe()))
        .collect::<Vec<_>>()
        .join("\n");

    Err(format!(
        "\n\nPlaybook expectation(s) failed:\n{}\n\nExpectations were:\n{}\n",
        errors.join("\n"),
        expectations_desc,
    ))
}

async fn delete_owned_journal_entries(
    client: &reqwest::Client,
    admin_url: &str,
    mapping_ids: &[String],
) {
    let owned: HashSet<&str> = mapping_ids.iter().map(|s| s.as_str()).collect();

    let resp = client
        .get(format!("{admin_url}/requests?limit=1000"))
        .send()
        .await;
    let events: Vec<ServeEvent> = match resp {
        Ok(r) if r.status().is_success() => {
            let text = r.text().await.unwrap_or_default();
            serde_json::from_str::<ServeEventsResponse>(&text)
                .map(|r| r.requests)
                .unwrap_or_default()
        }
        Ok(r) => {
            log::warn!(
                "ActivePlaybook::drop: fetch events got HTTP {} — skipping journal cleanup",
                r.status()
            );
            return;
        }
        Err(e) => {
            log::warn!("ActivePlaybook::drop: fetch events failed: {e} — skipping journal cleanup");
            return;
        }
    };

    for ev in events {
        let matches = event_stub_id(&ev).map(|id| owned.contains(id)).unwrap_or(false);
        if !matches {
            continue;
        }
        let url = format!("{admin_url}/requests/{id}", id = ev.id);
        match client.delete(&url).send().await {
            Ok(r) if r.status().is_success() => {
                log::debug!("ActivePlaybook::drop: deleted journal entry {}", ev.id);
            }
            Ok(r) => log::warn!(
                "ActivePlaybook::drop: delete journal entry {} got HTTP {}",
                ev.id,
                r.status()
            ),
            Err(e) => log::warn!(
                "ActivePlaybook::drop: delete journal entry {} failed: {e}",
                ev.id
            ),
        }
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

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.mapping = self.mapping.with_priority(priority);
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
            expectation: None,
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
    expectation: Option<ExpectedCount>,
}

impl PlaybookSequenceBuilder {
    pub fn then_return(mut self, response: ResponseDefinition) -> Self {
        self.responses.push(response);
        self
    }

    pub fn expect_called(mut self, count: u64) -> Self {
        self.expectation = Some(ExpectedCount::Exactly(count));
        self
    }

    pub fn expect_called_at_least(mut self, count: u64) -> Self {
        self.expectation = Some(ExpectedCount::AtLeast(count));
        self
    }

    pub fn expect_never_called(mut self) -> Self {
        self.expectation = Some(ExpectedCount::Never);
        self
    }

    pub fn into_playbook(mut self) -> Playbook {
        let method = self.mapping.method().to_string();
        let url_path = self.mapping.url_path().to_string();
        let mappings = self.mapping.will_return_sequence(self.responses);
        self.playbook.mappings.extend(mappings);
        if let Some(count) = self.expectation {
            self.playbook.expectations.push(Expectation {
                method,
                url_path,
                count,
            });
        }
        self.playbook
    }

    pub fn get(self, url_path: impl Into<String>) -> PlaybookMappingBuilder {
        self.into_playbook().get(url_path)
    }

    pub fn post(self, url_path: impl Into<String>) -> PlaybookMappingBuilder {
        self.into_playbook().post(url_path)
    }

    pub fn put(self, url_path: impl Into<String>) -> PlaybookMappingBuilder {
        self.into_playbook().put(url_path)
    }

    pub fn delete(self, url_path: impl Into<String>) -> PlaybookMappingBuilder {
        self.into_playbook().delete(url_path)
    }

    pub async fn run(self) -> ActivePlaybook {
        self.into_playbook().run().await
    }
}
