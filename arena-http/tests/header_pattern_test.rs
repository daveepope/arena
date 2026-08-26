use arena_http::HeaderPattern;

#[test]
fn equal_to_serializes_equal_to_key() {
    let json = serde_json::to_value(HeaderPattern::equal_to("value")).unwrap();
    assert_eq!(json["equalTo"], "value");
}

#[test]
fn matching_serializes_matches_key() {
    let json = serde_json::to_value(HeaderPattern::matching("^abc$")).unwrap();
    assert_eq!(json["matches"], "^abc$");
}
