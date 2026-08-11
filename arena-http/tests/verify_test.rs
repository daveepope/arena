use arena_http::{
    delete_requested_for, get_requested_for, post_requested_for, put_requested_for,
    HeaderPattern, RecordedRequest, RequestCriteria,
};
use std::collections::HashMap;

fn recorded_request(method: &str, url: &str, body: &str) -> RecordedRequest {
    RecordedRequest {
        url: url.to_string(),
        absolute_url: format!("http://localhost{url}"),
        method: method.to_string(),
        headers: HashMap::new(),
        body: body.to_string(),
        logged_date_string: "2026-01-01T00:00:00.000Z".to_string(),
    }
}

#[test]
fn get_requested_for_display_shows_method_and_path() {
    let criteria = get_requested_for("/things");
    assert_eq!(format!("{criteria}"), "GET /things");
}

#[test]
fn post_requested_for_display_shows_method_and_path() {
    assert_eq!(format!("{}", post_requested_for("/things")), "POST /things");
}

#[test]
fn put_requested_for_display_shows_method_and_path() {
    assert_eq!(format!("{}", put_requested_for("/things")), "PUT /things");
}

#[test]
fn delete_requested_for_display_shows_method_and_path() {
    assert_eq!(
        format!("{}", delete_requested_for("/things")),
        "DELETE /things"
    );
}

#[test]
fn with_header_display_appends_header_names() {
    let criteria =
        get_requested_for("/things").with_header("X-Trace", HeaderPattern::equal_to("abc"));
    let text = format!("{criteria}");
    assert!(text.starts_with("GET /things [headers: "));
    assert!(text.contains("X-Trace"));
}

#[test]
fn method_and_path_returns_both_parts() {
    let criteria: RequestCriteria = get_requested_for("/a");
    assert_eq!(criteria.method_and_path(), (Some("GET"), Some("/a")));
}

#[test]
fn recorded_request_display_no_body_shows_method_url() {
    let request = recorded_request("GET", "/things", "");
    assert_eq!(format!("{request}"), "GET /things");
}

#[test]
fn recorded_request_display_short_body_shows_full_body() {
    let request = recorded_request("POST", "/things", "{\"a\":1}");
    assert_eq!(format!("{request}"), "POST /things  body={\"a\":1}");
}

#[test]
fn recorded_request_display_long_body_truncates_with_count() {
    let long_body = "x".repeat(250);
    let request = recorded_request("POST", "/things", &long_body);
    let text = format!("{request}");
    assert!(text.starts_with("POST /things  body="));
    assert!(text.contains("...(250B total)"));
    assert!(!text.contains(&long_body));
}
