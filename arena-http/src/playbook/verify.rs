use crate::playbook::header_pattern::HeaderPattern;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCriteria {
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<HashMap<String, HeaderPattern>>,
}

impl RequestCriteria {
    fn new(method: &str, url_path: impl Into<String>) -> Self {
        Self {
            method: Some(method.to_string()),
            url_path: Some(url_path.into()),
            headers: None,
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, pattern: HeaderPattern) -> Self {
        self.headers
            .get_or_insert_with(HashMap::new)
            .insert(name.into(), pattern);
        self
    }

    pub fn method_and_path(&self) -> (Option<&str>, Option<&str>) {
        (self.method.as_deref(), self.url_path.as_deref())
    }

    pub(crate) fn has_header_criteria(&self) -> bool {
        self.headers
            .as_ref()
            .map(|h| !h.is_empty())
            .unwrap_or(false)
    }
}

impl fmt::Display for RequestCriteria {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let method = self.method.as_deref().unwrap_or("ANY");
        let path = self.url_path.as_deref().unwrap_or("*");
        write!(f, "{method} {path}")?;
        if let Some(headers) = &self.headers {
            let names: Vec<&String> = headers.keys().collect();
            write!(
                f,
                " [headers: {}]",
                names
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        }
        Ok(())
    }
}

pub fn get_requested_for(url_path: impl Into<String>) -> RequestCriteria {
    RequestCriteria::new("GET", url_path)
}

pub fn post_requested_for(url_path: impl Into<String>) -> RequestCriteria {
    RequestCriteria::new("POST", url_path)
}

pub fn put_requested_for(url_path: impl Into<String>) -> RequestCriteria {
    RequestCriteria::new("PUT", url_path)
}

pub fn delete_requested_for(url_path: impl Into<String>) -> RequestCriteria {
    RequestCriteria::new("DELETE", url_path)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedRequest {
    pub url: String,
    #[serde(default)]
    pub absolute_url: String,
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub logged_date_string: String,
}

const BODY_PREVIEW_MAX: usize = 200;

impl fmt::Display for RecordedRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.method, self.url)?;
        if !self.body.is_empty() {
            if self.body.len() <= BODY_PREVIEW_MAX {
                write!(f, "  body={}", self.body)?;
            } else {
                write!(
                    f,
                    "  body={}...({}B total)",
                    &self.body[..BODY_PREVIEW_MAX],
                    self.body.len()
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct FindResponse {
    pub requests: Vec<RecordedRequest>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ServeEventStubMapping {
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoggedRequest {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub method: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServeEvent {
    pub id: String,
    pub request: LoggedRequest,
    #[serde(default)]
    pub stub_mapping: Option<ServeEventStubMapping>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ServeEventsResponse {
    pub requests: Vec<ServeEvent>,
}

pub(crate) fn format_request_journal(requests: &[RecordedRequest]) -> String {
    if requests.is_empty() {
        return "  (no requests were received)".to_string();
    }
    requests
        .iter()
        .enumerate()
        .map(|(i, r)| format!("  {}. {r}", i + 1))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn event_stub_id(event: &ServeEvent) -> Option<&str> {
    event.stub_mapping.as_ref().and_then(|s| s.id.as_deref())
}

pub(crate) fn event_matches_criteria(event: &ServeEvent, criteria: &RequestCriteria) -> bool {
    let (method, path) = criteria.method_and_path();
    if let Some(expected_method) = method {
        if !event.request.method.eq_ignore_ascii_case(expected_method) {
            return false;
        }
    }
    if let Some(expected_path) = path {
        let logged_path = event
            .request
            .url
            .split('?')
            .next()
            .unwrap_or(&event.request.url);
        if logged_path != expected_path {
            return false;
        }
    }
    true
}
