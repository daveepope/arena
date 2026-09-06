use arena::lifecycle::{panic_message, Fault, Subject};

#[test]
fn dependency_identifier_and_message_returns_dependency_subject() {
    let fault = Fault::dependency("postgres-1", "readiness check never passed");

    assert_eq!(fault.id, "postgres-1");
    assert_eq!(fault.subject, Subject::Dependency);
    assert_eq!(fault.message, "readiness check never passed");
    assert!(fault.faults.is_empty());
}

#[test]
fn constructors_each_subject_returns_matching_subject() {
    assert_eq!(Fault::arena("a", "m").subject, Subject::Arena);
    assert_eq!(Fault::dependency("d", "m").subject, Subject::Dependency);
    assert_eq!(Fault::component("c", "m").subject, Subject::Component);
    assert_eq!(Fault::playbook("p", "m").subject, Subject::Playbook);
}

#[test]
fn caused_by_child_fault_nests_under_parent() {
    let child = Fault::dependency("child", "connection refused");
    let parent = Fault::dependency("parent", "child never came up").caused_by(child.clone());

    assert_eq!(parent.faults.len(), 1);
    assert_eq!(parent.faults[0], child);
}

#[test]
fn flatten_nested_tree_returns_every_fault_depth_first() {
    let grandchild = Fault::dependency("grandchild", "no route to host");
    let child = Fault::dependency("child", "startup failed").caused_by(grandchild);
    let parent = Fault::dependency("parent", "child never came up").caused_by(child);

    let flattened = parent.flatten();

    let ids: Vec<&str> = flattened.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, vec!["parent", "child", "grandchild"]);
}

#[test]
fn caused_by_all_many_faults_nests_every_one() {
    let parent = Fault::arena("test-arena", "two dependencies failed").caused_by_all(vec![
        Fault::dependency("one", "first"),
        Fault::dependency("two", "second"),
    ]);

    assert_eq!(parent.faults.len(), 2);
    assert_eq!(parent.flatten().len(), 3);
}

#[test]
fn timestamp_any_fault_returns_utc_rfc3339_with_milliseconds() {
    let stamped = Fault::dependency("postgres-1", "boom").timestamp();

    assert!(stamped.ends_with('Z'), "expected UTC suffix in {stamped}");
    assert_eq!(stamped.len(), 24, "expected millisecond precision in {stamped}");
}

#[test]
fn display_nested_fault_includes_subject_id_and_cause() {
    let fault = Fault::component("api", "container exited")
        .caused_by(Fault::dependency("postgres-1", "readiness check never passed"));

    let rendered = fault.to_string();

    assert!(rendered.contains("component 'api': container exited"));
    assert!(rendered.contains("caused by"));
    assert!(rendered.contains("dependency 'postgres-1'"));
}

#[test]
fn panic_message_string_payload_returns_the_string() {
    let payload: Box<dyn std::any::Any + Send> = Box::new("static panic".to_string());

    assert_eq!(panic_message(payload.as_ref()), "static panic");
}

#[test]
fn panic_message_str_payload_returns_the_str() {
    let payload: Box<dyn std::any::Any + Send> = Box::new("borrowed panic");

    assert_eq!(panic_message(payload.as_ref()), "borrowed panic");
}

#[test]
fn panic_message_unknown_payload_returns_placeholder() {
    let payload: Box<dyn std::any::Any + Send> = Box::new(7u8);

    assert_eq!(panic_message(payload.as_ref()), "unknown panic payload");
}

#[test]
fn from_panic_string_payload_returns_fault_carrying_the_panic_text() {
    let payload: Box<dyn std::any::Any + Send> = Box::new("boom".to_string());

    let fault = Fault::from_panic("orders-postgres", Subject::Dependency, payload.as_ref());

    assert_eq!(fault.id, "orders-postgres");
    assert_eq!(fault.subject, Subject::Dependency);
    assert_eq!(fault.message, "boom");
    assert!(fault.faults.is_empty());
}

#[test]
fn from_panic_unknown_payload_returns_fault_with_placeholder_message() {
    let payload: Box<dyn std::any::Any + Send> = Box::new(7u8);

    let fault = Fault::from_panic("api", Subject::Component, payload.as_ref());

    assert_eq!(fault.message, "unknown panic payload");
}

#[test]
fn serialize_fault_returns_subject_token_and_timestamp() {
    let fault = Fault::dependency("orders-postgres", "readiness check failed");

    let value = serde_json::to_value(&fault).expect("fault serializes");

    assert_eq!(value["id"], "orders-postgres");
    assert_eq!(value["subject"], "dependency");
    assert_eq!(value["message"], "readiness check failed");
    assert_eq!(value["at"], fault.timestamp());
    assert_eq!(value["faults"].as_array().unwrap().len(), 0);
}

#[test]
fn serialize_nested_fault_returns_nested_causes() {
    let fault = Fault::component("api", "child dependency failed to start")
        .caused_by(Fault::dependency("orders-postgres", "readiness check failed"));

    let value = serde_json::to_value(&fault).expect("fault serializes");

    assert_eq!(value["faults"][0]["id"], "orders-postgres");
    assert_eq!(value["faults"][0]["subject"], "dependency");
}

#[test]
fn serialize_every_subject_returns_its_as_str_token() {
    let subjects = [
        Subject::Arena,
        Subject::Dependency,
        Subject::Component,
        Subject::Playbook,
    ];

    for subject in subjects {
        assert_eq!(
            serde_json::to_value(subject).unwrap(),
            serde_json::json!(subject.as_str())
        );
    }
}

#[test]
fn display_two_level_cause_chain_indents_each_level() {
    let fault = Fault::arena("orders", "open faulted")
        .caused_by(Fault::component("api", "child dependency failed to start").caused_by(
            Fault::dependency("orders-postgres", "readiness check failed"),
        ));

    let rendered = fault.to_string();

    assert!(rendered.contains("\n  caused by "), "{rendered}");
    assert!(rendered.contains("\n    caused by "), "{rendered}");
}

#[test]
fn from_panic_as_a_cause_keeps_the_headline_message_human_readable() {
    let payload: Box<dyn std::any::Any + Send> = Box::new("index out of bounds".to_string());
    let fault = Fault::dependency("orders-postgres", "failed to start").caused_by(
        Fault::from_panic("orders-postgres", Subject::Dependency, payload.as_ref()),
    );

    assert_eq!(fault.message, "failed to start");
    assert_eq!(fault.faults[0].message, "index out of bounds");
    assert!(!fault.message.contains("panic"));
}
