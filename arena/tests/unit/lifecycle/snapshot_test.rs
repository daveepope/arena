use arena::lifecycle::{
    aggregate_faults, ArenaLifecycleState, ArenaState, ComponentState, DependencyState, Fault,
    RunnableState,
};

fn dependency(id: &str, state: RunnableState, faults: Vec<Fault>) -> DependencyState {
    DependencyState::new(id, state, faults, Vec::new())
}

fn component(id: &str, state: RunnableState, faults: Vec<Fault>) -> ComponentState {
    ComponentState::new(id, state, faults, Vec::new())
}

fn stamped(mut fault: Fault, millis: i64) -> Fault {
    fault.at = chrono::DateTime::from_timestamp_millis(millis).expect("timestamp");
    fault
}

#[test]
fn has_faulted_subject_faulted_child_returns_true() {
    let child = dependency("child", RunnableState::Faulted, Vec::new());
    let parent = DependencyState::new("parent", RunnableState::Stopped, Vec::new(), vec![child]);

    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaTeardown,
        vec![parent],
        Vec::new(),
        Vec::new(),
    );

    assert!(state.has_faulted_subject());
}

#[test]
fn has_faulted_subject_all_stopped_returns_false() {
    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaTeardown,
        vec![dependency("postgres-1", RunnableState::Stopped, Vec::new())],
        vec![component("api", RunnableState::NotStarted, Vec::new())],
        Vec::new(),
    );

    assert!(!state.has_faulted_subject());
}

#[test]
fn terminal_state_faulted_component_returns_arena_faulted() {
    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaTeardown,
        Vec::new(),
        vec![component("api", RunnableState::Faulted, Vec::new())],
        Vec::new(),
    );

    assert_eq!(state.terminal_state(), ArenaLifecycleState::ArenaFaulted);
}

#[test]
fn terminal_state_recorded_fault_only_returns_arena_faulted() {
    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaTeardown,
        vec![dependency("postgres-1", RunnableState::Stopped, Vec::new())],
        Vec::new(),
        vec![Fault::playbook("seed", "seed data rejected")],
    );

    assert_eq!(state.terminal_state(), ArenaLifecycleState::ArenaFaulted);
}

#[test]
fn terminal_state_clean_teardown_returns_arena_closed() {
    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaTeardown,
        vec![dependency("postgres-1", RunnableState::Stopped, Vec::new())],
        vec![component("api", RunnableState::Stopped, Vec::new())],
        Vec::new(),
    );

    assert_eq!(state.terminal_state(), ArenaLifecycleState::ArenaClosed);
}

#[test]
fn aggregate_faults_multiple_subjects_orders_by_timestamp() {
    let late = stamped(Fault::component("api", "late"), 3_000);
    let early = stamped(Fault::dependency("postgres-1", "early"), 1_000);
    let middle = stamped(Fault::arena("test-arena", "middle"), 2_000);

    let faults = aggregate_faults(
        &[dependency("postgres-1", RunnableState::Stopped, vec![early])],
        &[component("api", RunnableState::Stopped, vec![late])],
        vec![middle],
    );

    let messages: Vec<&str> = faults.iter().map(|f| f.message.as_str()).collect();
    assert_eq!(messages, vec!["early", "middle", "late"]);
}

#[test]
fn aggregate_faults_subject_fault_also_recorded_by_arena_returns_it_once() {
    let fault = stamped(Fault::dependency("postgres-1", "readiness"), 1_000);

    let faults = aggregate_faults(
        &[dependency(
            "postgres-1",
            RunnableState::Stopped,
            vec![fault.clone()],
        )],
        &[],
        vec![fault],
    );

    assert_eq!(faults.len(), 1);
}

#[test]
fn aggregate_faults_nested_children_collects_every_level() {
    let grandchild = DependencyState::new(
        "grandchild",
        RunnableState::Stopped,
        vec![stamped(Fault::dependency("grandchild", "deep"), 1_000)],
        Vec::new(),
    );
    let child = DependencyState::new(
        "child",
        RunnableState::Stopped,
        vec![stamped(Fault::dependency("child", "middle"), 2_000)],
        vec![grandchild],
    );
    let parent = DependencyState::new("parent", RunnableState::Stopped, Vec::new(), vec![child]);

    let faults = aggregate_faults(&[parent], &[], Vec::new());

    assert_eq!(faults.len(), 2);
}

#[test]
fn dependency_nested_identifier_returns_grandchild() {
    let grandchild = DependencyState::new("grandchild", RunnableState::Stopped, Vec::new(), Vec::new());
    let child = DependencyState::new(
        "child",
        RunnableState::Stopped,
        Vec::new(),
        vec![grandchild],
    );
    let parent = DependencyState::new("parent", RunnableState::Stopped, Vec::new(), vec![child]);

    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaOpen,
        vec![parent],
        Vec::new(),
        Vec::new(),
    );

    assert_eq!(state.dependency("grandchild").map(|d| d.id.as_str()), Some("grandchild"));
    assert!(state.dependency("missing").is_none());
}

#[test]
fn component_nested_identifier_returns_child() {
    let child = ComponentState::new("worker", RunnableState::Started, Vec::new(), Vec::new());
    let parent = ComponentState::new("api", RunnableState::Started, Vec::new(), vec![child]);

    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaOpen,
        Vec::new(),
        vec![parent],
        Vec::new(),
    );

    assert_eq!(state.component("worker").map(|c| c.id.as_str()), Some("worker"));
    assert!(state.component("missing").is_none());
}

#[test]
fn display_faulted_arena_lists_subjects_and_faults() {
    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaFaulted,
        vec![dependency(
            "postgres-1",
            RunnableState::Faulted,
            vec![Fault::dependency("postgres-1", "readiness check never passed")],
        )],
        vec![component("api", RunnableState::NotStarted, Vec::new())],
        Vec::new(),
    );

    let rendered = state.to_string();

    assert!(rendered.contains("arena 'test-arena' is arena_faulted"));
    assert!(rendered.contains("\n  dependencies:\n    'postgres-1': faulted"));
    assert!(rendered.contains("\n  components:\n    'api': not_started"));
    assert!(rendered.contains("\n  faults:\n    ["));
    assert!(rendered.contains("readiness check never passed"));
}

#[test]
fn display_nested_children_indents_each_level() {
    let child = DependencyState::new(
        "postgres-seed",
        RunnableState::Stopped,
        Vec::new(),
        Vec::new(),
    );
    let parent = DependencyState::new(
        "postgres-1",
        RunnableState::Faulted,
        Vec::new(),
        vec![child],
    );
    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaFaulted,
        vec![parent],
        Vec::new(),
        Vec::new(),
    );

    let rendered = state.to_string();

    assert!(rendered.contains("\n    'postgres-1': faulted"));
    assert!(rendered.contains("\n      'postgres-seed': stopped"));
}

#[test]
fn display_healthy_arena_omits_empty_sections() {
    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaOpen,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );

    let rendered = state.to_string();

    assert!(!rendered.contains("dependencies:"));
    assert!(!rendered.contains("components:"));
    assert!(!rendered.contains("faults:"));
}

#[test]
fn serialize_faulted_state_returns_every_section() {
    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaFaulted,
        vec![dependency(
            "postgres-1",
            RunnableState::Faulted,
            vec![Fault::dependency("postgres-1", "readiness check failed")],
        )],
        vec![component("api", RunnableState::NotStarted, Vec::new())],
        Vec::new(),
    );

    let value = serde_json::to_value(&state).expect("state serializes");

    assert_eq!(value["id"], "test-arena");
    assert_eq!(value["state"], "arena_faulted");
    assert_eq!(value["dependencies"][0]["id"], "postgres-1");
    assert_eq!(value["dependencies"][0]["state"], "faulted");
    assert_eq!(value["dependencies"][0]["children"].as_array().unwrap().len(), 0);
    assert_eq!(value["components"][0]["state"], "not_started");
    assert_eq!(value["faults"][0]["subject"], "dependency");
    assert_eq!(value["faults"][0]["message"], "readiness check failed");
}

#[test]
fn serialize_state_at_returns_the_display_timestamp() {
    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaOpen,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );

    let value = serde_json::to_value(&state).expect("state serializes");

    assert_eq!(value["at"], state.timestamp());
}

#[test]
fn serialize_nested_children_returns_the_full_tree() {
    let grandchild = DependencyState::new("seed", RunnableState::Stopped, Vec::new(), Vec::new());
    let child = DependencyState::new(
        "postgres-1",
        RunnableState::Stopped,
        Vec::new(),
        vec![grandchild],
    );
    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaClosed,
        vec![child],
        Vec::new(),
        Vec::new(),
    );

    let value = serde_json::to_value(&state).expect("state serializes");

    assert_eq!(value["dependencies"][0]["children"][0]["id"], "seed");
}

#[test]
fn timestamp_any_state_returns_utc_rfc3339_with_milliseconds() {
    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaOpen,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );

    let stamped = state.timestamp();

    assert!(stamped.ends_with('Z'), "expected UTC suffix in {stamped}");
    assert_eq!(stamped.len(), 24, "expected millisecond precision in {stamped}");
}

#[test]
fn display_fault_cause_inside_state_indents_under_its_fault() {
    let state = ArenaState::new(
        "test-arena",
        ArenaLifecycleState::ArenaFaulted,
        Vec::new(),
        vec![component(
            "api",
            RunnableState::NotStarted,
            vec![Fault::component("api", "child dependency failed to start")
                .caused_by(Fault::dependency("postgres-1", "readiness check failed"))],
        )],
        Vec::new(),
    );

    let rendered = state.to_string();

    assert!(
        rendered.contains("\n      caused by [") ,
        "cause should sit under its fault, got {rendered}"
    );
}
