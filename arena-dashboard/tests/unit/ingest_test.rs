use arena_dashboard::ingest::parse;

#[test]
fn parse_minimalvalidjson_returnsarenastate() {
    let body = r#"{"id":"my-arena","state":"arena_open","at":"2026-01-01T00:00:00.000Z"}"#;

    let parsed = parse(body).expect("valid minimal json should parse");

    assert_eq!(parsed.id, "my-arena");
    assert_eq!(parsed.state, "arena_open");
    assert!(parsed.dependencies.is_empty());
    assert!(parsed.components.is_empty());
    assert!(parsed.faults.is_empty());
}

#[test]
fn parse_nesteddependencieswithfaults_returnspopulatedtree() {
    let body = r#"{
        "id":"my-arena",
        "state":"dependencies_starting",
        "at":"2026-01-01T00:00:00.000Z",
        "dependencies":[
            {
                "id":"postgres",
                "state":"faulted",
                "faults":[{"id":"f1","subject":"dependency","message":"boom","at":"2026-01-01T00:00:00.000Z","faults":[]}],
                "children":[]
            }
        ]
    }"#;

    let parsed = parse(body).expect("nested json should parse");

    assert_eq!(parsed.dependencies.len(), 1);
    let dep = &parsed.dependencies[0];
    assert_eq!(dep.id, "postgres");
    assert_eq!(dep.state, "faulted");
    assert_eq!(dep.faults.len(), 1);
    assert_eq!(dep.faults[0].message, "boom");
}

#[test]
fn parse_malformedjson_returnserror() {
    let result = parse("{not valid json");

    assert!(result.is_err());
}

#[test]
fn parse_missingrequiredfield_returnserror() {
    let body = r#"{"state":"arena_open","at":"2026-01-01T00:00:00.000Z"}"#;

    let result = parse(body);

    assert!(result.is_err());
}

#[test]
fn parse_scoped_playbook_fullpayload_isparsed() {
    let body = r#"{"arena_id":"arena-orders","playbook_id":"reset-orders","playbook_type":"managed","kind":"scoped","state":"started","at":"2026-10-18T00:00:00.000Z","faults":[]}"#;

    let parsed = arena_dashboard::ingest::parse_scoped_playbook(body).expect("payload should parse");

    assert_eq!(parsed.arena_id, "arena-orders");
    assert_eq!(parsed.playbook_id, "reset-orders");
    assert_eq!(parsed.playbook_type, "managed");
    assert_eq!(parsed.kind, "scoped");
    assert_eq!(parsed.state, "started");
    assert_eq!(parsed.at, "2026-10-18T00:00:00.000Z");
    assert!(parsed.faults.is_empty());
}

#[test]
fn parse_scoped_playbook_withoutfaults_defaultstoempty() {
    let body = r#"{"arena_id":"arena-orders","playbook_id":"reset-orders","playbook_type":"managed","kind":"startup","state":"stopped","at":"2026-10-18T00:00:00.000Z"}"#;

    let parsed = arena_dashboard::ingest::parse_scoped_playbook(body).expect("payload should parse");

    assert!(parsed.faults.is_empty());
}

#[test]
fn parse_scoped_playbook_missingplaybookid_isrejected() {
    let body = r#"{"arena_id":"arena-orders","state":"started"}"#;

    assert!(arena_dashboard::ingest::parse_scoped_playbook(body).is_err());
}

#[test]
fn parse_scoped_playbook_withoutplaybooktype_leavesitempty() {
    let body = r#"{"arena_id":"arena-orders","playbook_id":"reset-orders","kind":"scoped","state":"started","at":"2026-10-18T00:00:00.000Z"}"#;

    let parsed = arena_dashboard::ingest::parse_scoped_playbook(body).expect("payload should parse");

    assert_eq!(parsed.playbook_type, "");
}

#[test]
fn parse_scoped_playbook_missingkind_isrejected() {
    let body = r#"{"arena_id":"arena-orders","playbook_id":"reset-orders","state":"started","at":"2026-10-18T00:00:00.000Z"}"#;

    assert!(arena_dashboard::ingest::parse_scoped_playbook(body).is_err());
}

