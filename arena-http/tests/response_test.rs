use arena_http::{
    a_response, bad_request, created, no_content, not_found, ok, ok_json, server_error, status,
    unauthorized, ResponseDefinition,
};

fn to_json(response: ResponseDefinition) -> serde_json::Value {
    serde_json::to_value(response).expect("response serializes")
}

#[test]
fn a_response_default_status_200() {
    let json = to_json(a_response());
    assert_eq!(json["status"], 200);
    assert!(json.get("jsonBody").is_none());
}

#[test]
fn ok_status_200() {
    assert_eq!(to_json(ok())["status"], 200);
}

#[test]
fn ok_json_body_set_includes_json_body() {
    let json = to_json(ok_json(serde_json::json!({"a": 1})));
    assert_eq!(json["status"], 200);
    assert_eq!(json["jsonBody"], serde_json::json!({"a": 1}));
}

#[test]
fn created_status_201() {
    assert_eq!(to_json(created())["status"], 201);
}

#[test]
fn no_content_status_204() {
    assert_eq!(to_json(no_content())["status"], 204);
}

#[test]
fn bad_request_status_400() {
    assert_eq!(to_json(bad_request())["status"], 400);
}

#[test]
fn unauthorized_status_401() {
    assert_eq!(to_json(unauthorized())["status"], 401);
}

#[test]
fn not_found_status_404() {
    assert_eq!(to_json(not_found())["status"], 404);
}

#[test]
fn server_error_status_500() {
    assert_eq!(to_json(server_error())["status"], 500);
}

#[test]
fn status_custom_code_sets_status() {
    assert_eq!(to_json(status(418))["status"], 418);
}

#[test]
fn with_status_override_sets_status() {
    let json = to_json(ok().with_status(299));
    assert_eq!(json["status"], 299);
}

#[test]
fn with_header_pair_adds_entry() {
    let json = to_json(ok().with_header("X-Test", "value"));
    assert_eq!(json["headers"]["X-Test"], "value");
}

#[test]
fn with_fixed_delay_ms_sets_delay_field() {
    let json = to_json(ok().with_fixed_delay_ms(250));
    assert_eq!(json["fixedDelayMilliseconds"], 250);
}

#[test]
fn with_uniform_random_delay_ms_sets_distribution() {
    let json = to_json(ok().with_uniform_random_delay_ms(10, 50));
    assert_eq!(json["delayDistribution"]["type"], "uniform");
    assert_eq!(json["delayDistribution"]["lower"], 10);
    assert_eq!(json["delayDistribution"]["upper"], 50);
}

#[test]
fn builder_chained_options_all_fields_present() {
    let json = to_json(
        status(202)
            .with_json_body(serde_json::json!({"ok": true}))
            .with_header("X-Trace", "abc")
            .with_fixed_delay_ms(5),
    );
    assert_eq!(json["status"], 202);
    assert_eq!(json["jsonBody"], serde_json::json!({"ok": true}));
    assert_eq!(json["headers"]["X-Trace"], "abc");
    assert_eq!(json["fixedDelayMilliseconds"], 5);
}
