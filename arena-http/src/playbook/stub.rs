use serde::Serialize;
use std::collections::HashMap;
use crate::playbook::header_pattern::HeaderPattern;
use crate::playbook::response::ResponseDefinition;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StubMapping {
    request: RequestPattern,
    response: ResponseDefinition,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scenario_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required_scenario_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_scenario_state: Option<String>,
}

impl StubMapping {
    pub fn method(&self) -> &str {
        &self.request.method
    }

    pub fn url_path(&self) -> &str {
        &self.request.url_path
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestPattern {
    method: String,
    url_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<HashMap<String, HeaderPattern>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body_patterns: Option<Vec<BodyPattern>>,
}

#[derive(Debug, Clone)]
enum BodyPattern {
    EqualToJson(serde_json::Value),
    Contains(String),
}

impl Serialize for BodyPattern {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Self::EqualToJson(val) => {
                let json_str = serde_json::to_string(val).unwrap_or_default();
                map.serialize_entry("equalToJson", &json_str)?;
            }
            Self::Contains(substring) => {
                map.serialize_entry("contains", substring)?;
            }
        }
        map.end()
    }
}

pub struct MappingBuilder {
    method: String,
    url_path: String,
    headers: Option<HashMap<String, HeaderPattern>>,
    body_patterns: Option<Vec<BodyPattern>>,
    priority: Option<u32>,
    scenario_name: Option<String>,
    required_scenario_state: Option<String>,
    new_scenario_state: Option<String>,
}

impl MappingBuilder {
    pub(crate) fn method(&self) -> &str {
        &self.method
    }

    pub(crate) fn url_path(&self) -> &str {
        &self.url_path
    }

    pub(crate) fn new(method: &str, url_path: impl Into<String>) -> Self {
        Self {
            method: method.to_string(),
            url_path: url_path.into(),
            headers: None,
            body_patterns: None,
            priority: None,
            scenario_name: None,
            required_scenario_state: None,
            new_scenario_state: None,
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn with_header(mut self, name: impl Into<String>, pattern: HeaderPattern) -> Self {
        self.headers
            .get_or_insert_with(HashMap::new)
            .insert(name.into(), pattern);
        self
    }

    pub fn with_request_body(mut self, body: serde_json::Value) -> Self {
        self.body_patterns
            .get_or_insert_with(Vec::new)
            .push(BodyPattern::EqualToJson(body));
        self
    }

    pub fn with_request_body_containing(mut self, substring: impl Into<String>) -> Self {
        self.body_patterns
            .get_or_insert_with(Vec::new)
            .push(BodyPattern::Contains(substring.into()));
        self
    }

    pub fn in_scenario(mut self, name: impl Into<String>) -> Self {
        self.scenario_name = Some(name.into());
        self
    }

    pub fn when_state_is(mut self, state: impl Into<String>) -> Self {
        self.required_scenario_state = Some(state.into());
        self
    }

    pub fn will_set_state_to(mut self, state: impl Into<String>) -> Self {
        self.new_scenario_state = Some(state.into());
        self
    }

    pub fn will_return(self, response: ResponseDefinition) -> StubMapping {
        let required_scenario_state = match (&self.scenario_name, self.required_scenario_state) {
            (Some(_), None) => Some("Started".to_string()),
            (_, state) => state,
        };

        StubMapping {
            request: RequestPattern {
                method: self.method,
                url_path: self.url_path,
                headers: self.headers,
                body_patterns: self.body_patterns,
            },
            response,
            priority: self.priority,
            scenario_name: self.scenario_name,
            required_scenario_state,
            new_scenario_state: self.new_scenario_state,
        }
    }

    pub(crate) fn will_return_sequence(self, responses: Vec<ResponseDefinition>) -> Vec<StubMapping> {
        if responses.len() <= 1 {
            return vec![self.will_return(responses.into_iter().next().expect("at least one response required"))];
        }

        let scenario_name = format!("__seq_{}_{}", self.method, self.url_path);
        let request = RequestPattern {
            method: self.method,
            url_path: self.url_path,
            headers: self.headers,
            body_patterns: self.body_patterns,
        };
        let priority = self.priority;

        let last = responses.len() - 1;
        responses
            .into_iter()
            .enumerate()
            .map(|(i, response)| {
                let required = if i == 0 {
                    "Started".to_string()
                } else {
                    format!("step-{i}")
                };
                let next = if i < last {
                    Some(format!("step-{}", i + 1))
                } else {
                    None
                };
                StubMapping {
                    request: request.clone(),
                    response,
                    priority,
                    scenario_name: Some(scenario_name.clone()),
                    required_scenario_state: Some(required),
                    new_scenario_state: next,
                }
            })
            .collect()
    }
}

pub fn get(url_path: impl Into<String>) -> MappingBuilder {
    MappingBuilder::new("GET", url_path)
}

pub fn post(url_path: impl Into<String>) -> MappingBuilder {
    MappingBuilder::new("POST", url_path)
}

pub fn put(url_path: impl Into<String>) -> MappingBuilder {
    MappingBuilder::new("PUT", url_path)
}

pub fn delete(url_path: impl Into<String>) -> MappingBuilder {
    MappingBuilder::new("DELETE", url_path)
}
