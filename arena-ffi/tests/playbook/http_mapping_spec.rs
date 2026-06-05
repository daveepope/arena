use arena_ffi::dependency::http::mapping::MappingSpec;

#[test]
fn mapping_spec_deserializes_delay_distribution() {
    let json = r#"{
        "method": "GET",
        "url_path": "/api/x",
        "response": {
            "status": 200,
            "delay_distribution": { "type": "uniform", "lower": 10, "upper": 50 }
        }
    }"#;
    let spec: MappingSpec = serde_json::from_str(json).unwrap();
    assert_eq!(spec.response.as_ref().unwrap().status, 200);
    assert!(spec.response.as_ref().unwrap().delay_distribution.is_some());
}

#[test]
fn mapping_spec_deserializes_body_contains_pattern() {
    let json = r#"{
        "method": "POST",
        "url_path": "/api/x",
        "body_patterns": [{ "contains": "ignition" }],
        "response": { "status": 200 }
    }"#;
    let spec: MappingSpec = serde_json::from_str(json).unwrap();
    assert_eq!(spec.body_patterns.as_ref().unwrap().len(), 1);
}

#[test]
fn mapping_spec_then_return_style_responses_array_deserializes() {
    let specs: Vec<MappingSpec> = serde_json::from_str(
        r#"[{
            "method": "GET",
            "url_path": "/api/altitude",
            "responses": [
                { "status": 500 },
                { "status": 503 },
                { "status": 200, "json_body": { "altitude_km": 185 } }
            ]
        }]"#,
    )
    .unwrap();
    assert_eq!(specs[0].responses.as_ref().unwrap().len(), 3);
}

#[test]
fn mapping_spec_expect_with_single_response_deserializes() {
    let specs: Vec<MappingSpec> = serde_json::from_str(
        r#"[{
            "method": "POST",
            "url_path": "/api/x",
            "response": { "status": 500 },
            "expect": { "kind": "at_least", "count": 1 }
        }]"#,
    )
    .unwrap();
    assert!(specs[0].expect.is_some());
}

#[test]
fn mapping_spec_missing_response_returns_error() {
    let spec: MappingSpec =
        serde_json::from_str(r#"{ "method": "GET", "url_path": "/api/x" }"#).unwrap();
    match spec.resolved_responses() {
        Err(msg) => assert!(msg.contains("requires response")),
        Ok(_) => panic!("expected mapping without response to fail"),
    }
}

#[test]
fn mapping_spec_scenario_fields_deserialize() {
    let spec: MappingSpec = serde_json::from_str(
        r#"{
            "method": "GET",
            "url_path": "/api/x",
            "scenario_name": "launch",
            "when_state_is": "flight",
            "will_set_state_to": "orbit",
            "response": { "status": 200 }
        }"#,
    )
    .unwrap();
    assert_eq!(spec.scenario_name.as_deref(), Some("launch"));
    assert_eq!(spec.when_state_is.as_deref(), Some("flight"));
    assert_eq!(spec.will_set_state_to.as_deref(), Some("orbit"));
}

#[test]
fn mapping_spec_header_equal_to_deserializes() {
    let json = r#"{
        "method": "POST",
        "url_path": "/api/x",
        "headers": { "Authorization": { "equal_to": "Bearer token" } },
        "response": { "status": 200 }
    }"#;
    let spec: MappingSpec = serde_json::from_str(json).unwrap();
    assert!(spec.headers.as_ref().unwrap().contains_key("Authorization"));
    let raw: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(
        raw["headers"]["Authorization"]["equal_to"],
        "Bearer token"
    );
}

#[test]
fn mapping_spec_header_matches_deserializes() {
    let json = r#"{
        "method": "POST",
        "url_path": "/api/x",
        "headers": { "X-Trace": { "matches": "trace-[0-9]+" } },
        "response": { "status": 200 }
    }"#;
    let spec: MappingSpec = serde_json::from_str(json).unwrap();
    assert!(spec.headers.as_ref().unwrap().contains_key("X-Trace"));
    let raw: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(raw["headers"]["X-Trace"]["matches"], "trace-[0-9]+");
}

#[test]
fn mapping_spec_priority_deserializes() {
    let spec: MappingSpec = serde_json::from_str(
        r#"{
            "method": "GET",
            "url_path": "/api/x",
            "priority": 3,
            "response": { "status": 200 }
        }"#,
    )
    .unwrap();
    assert_eq!(spec.priority, Some(3));
}

#[test]
fn mapping_spec_fixed_delay_ms_deserializes() {
    let spec: MappingSpec = serde_json::from_str(
        r#"{
            "method": "GET",
            "url_path": "/api/x",
            "response": { "status": 200, "fixed_delay_ms": 25 }
        }"#,
    )
    .unwrap();
    assert_eq!(spec.response.as_ref().unwrap().fixed_delay_ms, Some(25));
}

#[test]
fn mapping_spec_response_headers_deserialize() {
    let spec: MappingSpec = serde_json::from_str(
        r#"{
            "method": "POST",
            "url_path": "/api/x",
            "response": {
                "status": 201,
                "headers": { "Location": "/api/x/1" }
            }
        }"#,
    )
    .unwrap();
    let headers = spec.response.as_ref().unwrap().headers.as_ref().unwrap();
    assert_eq!(headers.get("Location").map(String::as_str), Some("/api/x/1"));
}

#[test]
fn mapping_spec_body_equal_to_json_deserializes() {
    let json = r#"{
        "method": "POST",
        "url_path": "/api/x",
        "body_patterns": [{ "equal_to_json": "{\"command\":\"ignition\"}" }],
        "response": { "status": 200 }
    }"#;
    let spec: MappingSpec = serde_json::from_str(json).unwrap();
    assert_eq!(spec.body_patterns.as_ref().unwrap().len(), 1);
    let raw: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(
        raw["body_patterns"][0]["equal_to_json"],
        "{\"command\":\"ignition\"}"
    );
}

#[test]
fn mapping_spec_will_return_in_sequence_responses_without_expect_deserializes() {
    let spec: MappingSpec = serde_json::from_str(
        r#"{
            "method": "GET",
            "url_path": "/api/x",
            "responses": [
                { "status": 500 },
                { "status": 503 },
                { "status": 200, "json_body": { "ok": true } }
            ]
        }"#,
    )
    .unwrap();
    assert!(spec.response.is_none());
    assert_eq!(spec.responses.as_ref().unwrap().len(), 3);
    assert!(spec.expect.is_none());
}
