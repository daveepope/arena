use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseDefinition {
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    json_body: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fixed_delay_milliseconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delay_distribution: Option<DelayDistribution>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DelayDistribution {
    #[serde(rename = "type")]
    distribution_type: String,
    lower: u64,
    upper: u64,
}

impl ResponseDefinition {
    pub fn new(status: u16) -> Self {
        Self {
            status,
            json_body: None,
            headers: None,
            fixed_delay_milliseconds: None,
            delay_distribution: None,
        }
    }

    pub fn with_status(mut self, code: u16) -> Self {
        self.status = code;
        self
    }

    pub fn with_json_body(mut self, body: serde_json::Value) -> Self {
        self.json_body = Some(body);
        self
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers
            .get_or_insert_with(HashMap::new)
            .insert(name.into(), value.into());
        self
    }

    pub fn with_fixed_delay_ms(mut self, ms: u64) -> Self {
        self.fixed_delay_milliseconds = Some(ms);
        self
    }

    pub fn with_uniform_random_delay_ms(mut self, lower: u64, upper: u64) -> Self {
        self.delay_distribution = Some(DelayDistribution {
            distribution_type: "uniform".to_string(),
            lower,
            upper,
        });
        self
    }
}

pub fn a_response() -> ResponseDefinition {
    ResponseDefinition::new(200)
}

pub fn ok() -> ResponseDefinition {
    ResponseDefinition::new(200)
}

pub fn ok_json(body: serde_json::Value) -> ResponseDefinition {
    ResponseDefinition::new(200).with_json_body(body)
}

pub fn created() -> ResponseDefinition {
    ResponseDefinition::new(201)
}

pub fn no_content() -> ResponseDefinition {
    ResponseDefinition::new(204)
}

pub fn bad_request() -> ResponseDefinition {
    ResponseDefinition::new(400)
}

pub fn unauthorized() -> ResponseDefinition {
    ResponseDefinition::new(401)
}

pub fn not_found() -> ResponseDefinition {
    ResponseDefinition::new(404)
}

pub fn server_error() -> ResponseDefinition {
    ResponseDefinition::new(500)
}

pub fn status(code: u16) -> ResponseDefinition {
    ResponseDefinition::new(code)
}
