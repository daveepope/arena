use std::collections::HashMap;

use arena_http::{
    HeaderPattern, Playbook as HttpPlaybook, PlaybookMappingBuilder, PlaybookSequenceBuilder,
    ResponseDefinition,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExpectSpec {
    Exactly { count: u64 },
    AtLeast { count: u64 },
    Never,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MappingSpec {
    pub method: String,
    pub url_path: String,
    #[serde(default)]
    pub priority: Option<u32>,
    #[serde(default)]
    pub scenario_name: Option<String>,
    #[serde(default)]
    pub when_state_is: Option<String>,
    #[serde(default)]
    pub will_set_state_to: Option<String>,
    #[serde(default)]
    pub headers: Option<HashMap<String, HeaderPatternSpec>>,
    #[serde(default)]
    pub body_patterns: Option<Vec<BodyPatternSpec>>,
    #[serde(default)]
    pub response: Option<ResponseSpec>,
    #[serde(default)]
    pub responses: Option<Vec<ResponseSpec>>,
    #[deprecated(
        note = "This API is deprecated and will be removed in a future release. Use response or responses instead."
    )]
    #[serde(default)]
    pub status: Option<u16>,
    #[deprecated(
        note = "This API is deprecated and will be removed in a future release. Use response or responses instead."
    )]
    #[serde(default)]
    pub json_body: Option<serde_json::Value>,
    #[serde(default)]
    pub expect: Option<ExpectSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeaderPatternSpec {
    #[serde(default)]
    equal_to: Option<String>,
    #[serde(default)]
    matches: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BodyPatternSpec {
    #[serde(default)]
    equal_to_json: Option<String>,
    #[serde(default)]
    contains: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseSpec {
    #[serde(default = "default_status")]
    pub status: u16,
    #[serde(default)]
    pub json_body: Option<serde_json::Value>,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub fixed_delay_ms: Option<u64>,
    #[serde(default)]
    pub delay_distribution: Option<DelayDistributionSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DelayDistributionSpec {
    #[serde(rename = "type")]
    distribution_type: String,
    lower: u64,
    upper: u64,
}

fn default_status() -> u16 {
    200
}

impl MappingSpec {
    #[allow(deprecated)]
    pub fn resolved_responses(&self) -> Result<Vec<ResponseSpec>, String> {
        if let Some(ref responses) = self.responses {
            if responses.is_empty() {
                return Err("responses must not be empty when provided".to_string());
            }
            return Ok(responses.clone());
        }
        if let Some(ref response) = self.response {
            return Ok(vec![response.clone()]);
        }
        if self.status.is_some() || self.json_body.is_some() {
            return Ok(vec![ResponseSpec {
                status: self.status.unwrap_or(200),
                json_body: self.json_body.clone(),
                headers: None,
                fixed_delay_ms: None,
                delay_distribution: None,
            }]);
        }
        Err("mapping requires response, responses, or status".to_string())
    }
}

pub(crate) fn response_def(spec: &ResponseSpec) -> ResponseDefinition {
    let mut r = ResponseDefinition::new(spec.status);
    if let Some(ref body) = spec.json_body {
        r = r.with_json_body(body.clone());
    }
    if let Some(ref headers) = spec.headers {
        for (name, value) in headers {
            r = r.with_header(name.clone(), value.clone());
        }
    }
    if let Some(ms) = spec.fixed_delay_ms {
        r = r.with_fixed_delay_ms(ms);
    }
    if let Some(ref dist) = spec.delay_distribution {
        if dist.distribution_type == "uniform" {
            r = r.with_uniform_random_delay_ms(dist.lower, dist.upper);
        }
    }
    r
}

fn header_pattern(spec: &HeaderPatternSpec) -> Result<HeaderPattern, String> {
    match (&spec.equal_to, &spec.matches) {
        (Some(value), None) => Ok(HeaderPattern::equal_to(value)),
        (None, Some(value)) => Ok(HeaderPattern::matching(value)),
        _ => Err("header pattern requires exactly one of equal_to or matches".to_string()),
    }
}

fn apply_body_pattern(
    builder: PlaybookMappingBuilder,
    pattern: &BodyPatternSpec,
) -> Result<PlaybookMappingBuilder, String> {
    match (&pattern.equal_to_json, &pattern.contains) {
        (Some(json), None) => {
            let value: serde_json::Value = serde_json::from_str(json).map_err(|e| {
                format!("body pattern equal_to_json is not valid JSON: {e}")
            })?;
            Ok(builder.with_request_body(value))
        }
        (None, Some(substring)) => Ok(builder.with_request_body_containing(substring.clone())),
        _ => Err(
            "body pattern requires exactly one of equal_to_json or contains".to_string(),
        ),
    }
}

pub(crate) fn apply_mapping_options(
    mut builder: PlaybookMappingBuilder,
    spec: &MappingSpec,
) -> Result<PlaybookMappingBuilder, String> {
    if let Some(priority) = spec.priority {
        builder = builder.with_priority(priority);
    }
    if let Some(ref name) = spec.scenario_name {
        builder = builder.in_scenario(name.clone());
    }
    if let Some(ref state) = spec.when_state_is {
        builder = builder.when_state_is(state.clone());
    }
    if let Some(ref state) = spec.will_set_state_to {
        builder = builder.will_set_state_to(state.clone());
    }
    if let Some(ref headers) = spec.headers {
        for (name, pattern_spec) in headers {
            builder = builder.with_header(name.clone(), header_pattern(pattern_spec)?);
        }
    }
    if let Some(ref patterns) = spec.body_patterns {
        for pattern in patterns {
            builder = apply_body_pattern(builder, pattern)?;
        }
    }
    Ok(builder)
}

fn start_mapping(playbook: HttpPlaybook, spec: &MappingSpec) -> Result<PlaybookMappingBuilder, String> {
    let builder = match spec.method.to_ascii_uppercase().as_str() {
        "GET" => playbook.get(&spec.url_path),
        "POST" => playbook.post(&spec.url_path),
        "PUT" => playbook.put(&spec.url_path),
        "DELETE" => playbook.delete(&spec.url_path),
        other => return Err(format!("unsupported HTTP method: {other}")),
    };
    apply_mapping_options(builder, spec)
}

pub(crate) fn apply_expect(
    seq: PlaybookSequenceBuilder,
    expect: &Option<ExpectSpec>,
) -> PlaybookSequenceBuilder {
    match expect {
        Some(ExpectSpec::Exactly { count }) => seq.expect_called(*count),
        Some(ExpectSpec::AtLeast { count }) => seq.expect_called_at_least(*count),
        Some(ExpectSpec::Never) => seq.expect_never_called(),
        None => seq,
    }
}

enum BuildState {
    Playbook(HttpPlaybook),
    Sequence(PlaybookSequenceBuilder),
}

fn apply_mapping_step(
    playbook: HttpPlaybook,
    spec: &MappingSpec,
) -> Result<BuildState, String> {
    let responses = spec.resolved_responses()?;
    let response_defs: Vec<ResponseDefinition> = responses.iter().map(response_def).collect();
    let builder = start_mapping(playbook, spec)?;

    if responses.len() > 1 && spec.expect.is_none() {
        return Ok(BuildState::Playbook(
            builder.will_return_in_sequence(response_defs),
        ));
    }

    let mut seq = builder.will_return(response_defs[0].clone());
    for response in response_defs.into_iter().skip(1) {
        seq = seq.then_return(response);
    }
    Ok(BuildState::Sequence(apply_expect(seq, &spec.expect)))
}

pub fn build_playbook_from_mappings(
    playbook: HttpPlaybook,
    mappings: &[MappingSpec],
) -> Result<HttpPlaybook, String> {
    if mappings.is_empty() {
        return Err("mappings must not be empty".to_string());
    }

    let mut state = apply_mapping_step(playbook, &mappings[0])?;
    for spec in mappings.iter().skip(1) {
        state = match state {
            BuildState::Playbook(pb) => apply_mapping_step(pb, spec)?,
            BuildState::Sequence(seq) => apply_mapping_step(seq.into_playbook(), spec)?,
        };
    }

    match state {
        BuildState::Playbook(pb) => Ok(pb),
        BuildState::Sequence(seq) => Ok(seq.into_playbook()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapping_spec_deserializes_flat_managed_status() {
        let json = r#"{
            "method": "POST",
            "url_path": "/api/x",
            "status": 500,
            "json_body": { "err": true }
        }"#;
        let spec: MappingSpec = serde_json::from_str(json).unwrap();
        let responses = spec.resolved_responses().unwrap();
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].status, 500);
    }

    #[test]
    fn response_def_fixed_delay_and_headers_apply_to_definition() {
        let spec = ResponseSpec {
            status: 201,
            json_body: None,
            headers: Some(HashMap::from([(
                "Location".to_string(),
                "/api/x/1".to_string(),
            )])),
            fixed_delay_ms: Some(40),
            delay_distribution: None,
        };
        let def = response_def(&spec);
        let json = serde_json::to_value(&def).unwrap();
        assert_eq!(json.get("status").and_then(|v| v.as_u64()), Some(201));
        assert_eq!(
            json.get("fixedDelayMilliseconds")
                .and_then(|v| v.as_u64()),
            Some(40)
        );
        assert_eq!(
            json.get("headers")
                .and_then(|v| v.get("Location"))
                .and_then(|v| v.as_str()),
            Some("/api/x/1")
        );
    }

    #[test]
    fn response_def_uniform_delay_distribution_applies_to_definition() {
        let spec = ResponseSpec {
            status: 200,
            json_body: None,
            headers: None,
            fixed_delay_ms: None,
            delay_distribution: Some(DelayDistributionSpec {
                distribution_type: "uniform".to_string(),
                lower: 5,
                upper: 15,
            }),
        };
        let def = response_def(&spec);
        let json = serde_json::to_value(&def).unwrap();
        assert_eq!(json["delayDistribution"]["type"], "uniform");
        assert_eq!(json["delayDistribution"]["lower"], 5);
        assert_eq!(json["delayDistribution"]["upper"], 15);
    }
}
