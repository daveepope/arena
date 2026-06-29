use arbiter::{AgentHarness, UnsupportedHarness};

#[test]
fn parse_cursor_returns_harness() {
    assert_eq!(AgentHarness::parse("cursor"), Ok(AgentHarness::Cursor));
}

#[test]
fn parse_mixed_case_and_spaces_returns_harness() {
    assert_eq!(AgentHarness::parse("  Cursor "), Ok(AgentHarness::Cursor));
}

#[test]
fn parse_unknown_returns_unsupported() {
    let error = AgentHarness::parse("claude").unwrap_err();
    assert_eq!(error, UnsupportedHarness("claude".to_string()));
}

#[test]
fn name_round_trips_through_parse() {
    let harness = AgentHarness::Cursor;
    assert_eq!(AgentHarness::parse(harness.name()), Ok(harness));
}
