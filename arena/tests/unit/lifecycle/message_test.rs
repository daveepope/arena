use arena::lifecycle::message::{self, Phase};
use arena::lifecycle::Subject;

#[test]
fn as_str_every_phase_returns_lifecycle_wording() {
    let cases = [
        (Phase::Starting, "starting"),
        (Phase::Stopping, "stopping"),
        (Phase::ForcedTeardown, "being forcibly stopped"),
        (Phase::RunningPlaybook, "running"),
    ];

    for (phase, expected) in cases {
        assert_eq!(phase.as_str(), expected, "phase {phase:?}");
    }
}

#[test]
fn readiness_failed_cause_returns_cause_after_prefix() {
    assert_eq!(
        message::readiness_failed("connection refused on 127.0.0.1:5432"),
        "readiness check failed: connection refused on 127.0.0.1:5432"
    );
}

#[test]
fn stop_did_not_complete_no_input_returns_fixed_wording() {
    assert_eq!(message::stop_did_not_complete(), "stop did not complete");
}

#[test]
fn child_start_failed_dependency_returns_dependency_wording() {
    assert_eq!(
        message::child_start_failed(Subject::Dependency),
        "child dependency failed to start"
    );
}

#[test]
fn child_start_failed_component_returns_component_wording() {
    assert_eq!(
        message::child_start_failed(Subject::Component),
        "child component failed to start"
    );
}

#[test]
fn forced_teardown_unconfirmed_no_input_returns_neutral_wording() {
    assert_eq!(
        message::forced_teardown_unconfirmed(),
        "forced teardown could not confirm the subject was removed"
    );
}

#[test]
fn panicked_while_starting_returns_phase_and_panic_text() {
    assert_eq!(
        message::panicked_while(Phase::Starting, "boom"),
        "panicked while starting: boom"
    );
}

#[test]
fn panicked_while_forced_teardown_returns_phase_and_panic_text() {
    assert_eq!(
        message::panicked_while(Phase::ForcedTeardown, "boom"),
        "panicked while being forcibly stopped: boom"
    );
}

#[test]
fn unexplained_after_teardown_running_subject_names_subject_and_state() {
    assert_eq!(
        message::unexplained_after_teardown(Subject::Dependency, "orders-postgres", "started"),
        "dependency 'orders-postgres' is started after forced teardown and reported no fault"
    );
}

#[test]
fn readiness_failed_for_target_returns_target_and_cause() {
    assert_eq!(
        message::readiness_failed_for_target("http://127.0.0.1:8080/health", "timed out"),
        "readiness check failed for target http://127.0.0.1:8080/health: timed out"
    );
}

#[test]
fn match_panicked_while_starting_returns_index_phase_and_panic_text() {
    assert_eq!(
        message::match_panicked_while(2, Phase::Starting, "boom"),
        "match 2 panicked while starting: boom"
    );
}
