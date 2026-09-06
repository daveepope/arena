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
