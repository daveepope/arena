use arena_http::{ok, HeaderPattern};

fn json(mapping: impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(mapping).expect("mapping serializes")
}

#[test]
fn get_free_fn_builds_get_mapping() {
    let mapping = arena_http::playbook::stub::get("/things").will_return(ok());
    assert_eq!(mapping.method(), "GET");
    assert_eq!(mapping.url_path(), "/things");
}

#[test]
fn post_free_fn_builds_post_mapping() {
    let mapping = arena_http::playbook::stub::post("/things").will_return(ok());
    assert_eq!(mapping.method(), "POST");
}

#[test]
fn put_free_fn_builds_put_mapping() {
    let mapping = arena_http::playbook::stub::put("/things").will_return(ok());
    assert_eq!(mapping.method(), "PUT");
}

#[test]
fn delete_free_fn_builds_delete_mapping() {
    let mapping = arena_http::playbook::stub::delete("/things").will_return(ok());
    assert_eq!(mapping.method(), "DELETE");
}

#[test]
fn will_return_scenario_no_state_defaults_started() {
    let mapping = arena_http::playbook::stub::get("/things")
        .in_scenario("my-scenario")
        .will_return(ok());
    let json = json(mapping);
    assert_eq!(json["requiredScenarioState"], "Started");
    assert_eq!(json["scenarioName"], "my-scenario");
}

#[test]
fn will_return_scenario_with_state_uses_given_state() {
    let mapping = arena_http::playbook::stub::get("/things")
        .in_scenario("my-scenario")
        .when_state_is("step-1")
        .will_set_state_to("step-2")
        .will_return(ok());
    let json = json(mapping);
    assert_eq!(json["requiredScenarioState"], "step-1");
    assert_eq!(json["newScenarioState"], "step-2");
}

#[test]
fn with_priority_sets_priority_field() {
    let mapping = arena_http::playbook::stub::get("/things")
        .with_priority(3)
        .will_return(ok());
    assert_eq!(json(mapping)["priority"], 3);
}

#[test]
fn with_header_sets_header_pattern() {
    let mapping = arena_http::playbook::stub::get("/things")
        .with_header("X-Trace", HeaderPattern::equal_to("abc"))
        .will_return(ok());
    let json = json(mapping);
    assert_eq!(json["request"]["headers"]["X-Trace"]["equalTo"], "abc");
}

#[test]
fn with_request_body_sets_equal_to_json_pattern() {
    let mapping = arena_http::playbook::stub::post("/things")
        .with_request_body(serde_json::json!({"a": 1}))
        .will_return(ok());
    let json = json(mapping);
    assert!(json["request"]["bodyPatterns"][0]["equalToJson"]
        .as_str()
        .unwrap()
        .contains("\"a\":1"));
}

#[test]
fn with_request_body_containing_sets_contains_pattern() {
    let mapping = arena_http::playbook::stub::post("/things")
        .with_request_body_containing("needle")
        .will_return(ok());
    let json = json(mapping);
    assert_eq!(json["request"]["bodyPatterns"][0]["contains"], "needle");
}
