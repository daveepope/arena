#[path = "../subjects.rs"]
mod subjects;

use arena::lifecycle::rule::LifecycleRule;
use arena::playbook::{PlaybookRule, PlaybookState};

#[test]
fn as_str_every_state_names_itself() {
    assert_eq!(PlaybookState::Registered.as_str(), "registered");
    assert_eq!(PlaybookState::Running.as_str(), "running");
    assert_eq!(PlaybookState::Stopped.as_str(), "stopped");
    assert_eq!(PlaybookState::Faulted.as_str(), "faulted");
}

#[test]
fn display_every_state_matches_its_name() {
    for state in [
        PlaybookState::Registered,
        PlaybookState::Running,
        PlaybookState::Stopped,
        PlaybookState::Faulted,
    ] {
        assert_eq!(state.to_string(), state.as_str());
    }
}

#[test]
fn serialize_every_state_is_snake_case() {
    for state in [
        PlaybookState::Registered,
        PlaybookState::Running,
        PlaybookState::Stopped,
        PlaybookState::Faulted,
    ] {
        let serialized = serde_json::to_string(&state).expect("serialize");

        assert_eq!(serialized, format!("\"{}\"", state.as_str()));
    }
}

#[test]
fn is_legal_playbook_run_and_stop_are_allowed() {
    use arena::playbook::PlaybookState::*;

    assert!(PlaybookRule.is_legal(Registered, Running));
    assert!(PlaybookRule.is_legal(Running, Stopped));
}

#[test]
fn is_legal_playbook_rerun_from_a_terminal_state_is_allowed() {
    use arena::playbook::PlaybookState::*;

    assert!(PlaybookRule.is_legal(Stopped, Running));
    assert!(PlaybookRule.is_legal(Faulted, Running));
}

#[test]
fn is_legal_playbook_fault_is_reachable_from_every_state() {
    use arena::playbook::PlaybookState::*;

    for from in [Registered, Running, Stopped, Faulted] {
        assert!(PlaybookRule.is_legal(from, Faulted), "{from} -> faulted");
    }
}

#[test]
fn is_legal_playbook_skipping_or_repeating_a_state_is_refused() {
    use arena::playbook::PlaybookState::*;

    assert!(!PlaybookRule.is_legal(Registered, Stopped));
    assert!(!PlaybookRule.is_legal(Registered, Registered));
    assert!(!PlaybookRule.is_legal(Running, Running));
    assert!(!PlaybookRule.is_legal(Stopped, Stopped));
    assert!(!PlaybookRule.is_legal(Stopped, Registered));
    assert!(!PlaybookRule.is_legal(Faulted, Stopped));
}

#[test]
fn is_legal_refused_transition_publishes_nothing() {
    let lifecycle = arena::playbook::PlaybookLifecycle::new("seed");
    let observed = subjects::observe_playbooks(
        &lifecycle,
        "arena-under-test",
        arena::playbook::PlaybookKind::Scoped,
    );
    let announced = observed.states();

    lifecycle.report(PlaybookState::Stopped);

    assert_eq!(observed.states(), announced);
    assert_eq!(lifecycle.state(), PlaybookState::Registered);
}

#[test]
fn exit_without_a_matching_run_reports_nothing_and_does_not_underflow() {
    let lifecycle = arena::playbook::PlaybookLifecycle::new("reset-orders");
    let observed = subjects::observe_playbooks(
        &lifecycle,
        "arena-under-test",
        arena::playbook::PlaybookKind::Scoped,
    );

    lifecycle.exit();
    lifecycle.exit();

    assert_eq!(observed.states(), vec![PlaybookState::Registered]);
    lifecycle.report_running();
    lifecycle.enter();
    lifecycle.exit();
    assert_eq!(lifecycle.state(), PlaybookState::Stopped);
}
