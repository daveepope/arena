#[path = "../subjects.rs"]
mod subjects;

use arena::lifecycle::{
    ArenaLifecycleState, ComponentState, DependencyState, Fault, ArenaLifecyclePublisher, RunnableState,
};
use std::sync::Arc;
use subjects::StateRecorder;

fn context_with_recorder() -> (ArenaLifecyclePublisher, Arc<StateRecorder>) {
    let recorder = Arc::new(StateRecorder::default());
    let context = ArenaLifecyclePublisher::new("test-arena", vec![recorder.clone()]);
    (context, recorder)
}

#[test]
fn new_context_starts_at_arena_created() {
    let (context, recorder) = context_with_recorder();

    assert_eq!(context.current(), ArenaLifecycleState::ArenaCreated);
    assert!(recorder.states().is_empty());
}

#[test]
fn transition_forward_states_advances_and_notifies_each_one() {
    let (context, recorder) = context_with_recorder();

    for state in [
        ArenaLifecycleState::ArenaStarting,
        ArenaLifecycleState::DependenciesStarting,
        ArenaLifecycleState::DependenciesStarted,
    ] {
        context.transition(state);
    }

    assert_eq!(
        recorder.states(),
        vec![
            ArenaLifecycleState::ArenaStarting,
            ArenaLifecycleState::DependenciesStarting,
            ArenaLifecycleState::DependenciesStarted,
        ]
    );
    assert_eq!(context.current(), ArenaLifecycleState::DependenciesStarted);
}

#[test]
fn transition_earlier_state_keeps_current_and_does_not_notify() {
    let (context, recorder) = context_with_recorder();

    context.transition(ArenaLifecycleState::ComponentsStarted);
    let snapshot = context.transition(ArenaLifecycleState::DependenciesStarting);

    assert_eq!(context.current(), ArenaLifecycleState::ComponentsStarted);
    assert_eq!(snapshot.state, ArenaLifecycleState::ComponentsStarted);
    assert_eq!(recorder.states(), vec![ArenaLifecycleState::ComponentsStarted]);
}

#[test]
fn transition_repeated_state_notifies_only_once() {
    let (context, recorder) = context_with_recorder();

    for _ in 0..3 {
        context.transition(ArenaLifecycleState::ArenaStarting);
    }

    assert_eq!(recorder.states(), vec![ArenaLifecycleState::ArenaStarting]);
}

#[test]
fn record_arena_fault_appears_in_next_snapshot() {
    let (context, _recorder) = context_with_recorder();
    context.record(Fault::arena("test-arena", "runtime unavailable"));

    let snapshot = context.transition(ArenaLifecycleState::ArenaStarting);

    assert_eq!(snapshot.faults.len(), 1);
    assert_eq!(snapshot.faults[0].message, "runtime unavailable");
    assert_eq!(context.recorded_faults().len(), 1);
}

#[test]
fn finish_clean_subjects_returns_arena_closed() {
    let (context, recorder) = context_with_recorder();
    context.transition(ArenaLifecycleState::ArenaTeardown);

    let slot = context.reserve_match();
    context.report_match(
        slot,
        vec![DependencyState::new(
            "postgres-1",
            RunnableState::Stopped,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )],
        Vec::new(),
    );

    let state = context.finish();

    assert_eq!(state.state, ArenaLifecycleState::ArenaClosed);
    assert_eq!(recorder.states().last(), Some(&ArenaLifecycleState::ArenaClosed));
}

#[test]
fn finish_faulted_subject_returns_arena_faulted() {
    let (context, recorder) = context_with_recorder();
    context.transition(ArenaLifecycleState::ArenaTeardown);

    let slot = context.reserve_match();
    context.report_match(
        slot,
        Vec::new(),
        vec![ComponentState::new(
            "api",
            RunnableState::Faulted,
            vec![Fault::component("api", "could not confirm removal")],
            Vec::new(),
        )],
    );

    let state = context.finish();

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert_eq!(recorder.states().last(), Some(&ArenaLifecycleState::ArenaFaulted));
}

#[test]
fn finish_recorded_fault_only_returns_arena_faulted() {
    let (context, _recorder) = context_with_recorder();
    context.record(Fault::playbook("seed", "seed data rejected"));

    let state = context.finish();

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
}

#[test]
fn arena_id_any_context_returns_the_configured_id() {
    let (context, _recorder) = context_with_recorder();

    assert_eq!(context.arena_id(), "test-arena");
}

#[test]
fn report_match_every_match_appears_in_one_snapshot() {
    let (context, recorder) = context_with_recorder();
    let first = context.reserve_match();
    let second = context.reserve_match();
    context.report_match(
        first,
        vec![DependencyState::new(
            "postgres-1",
            RunnableState::Started,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )],
        Vec::new(),
    );
    context.report_match(
        second,
        vec![DependencyState::new(
            "kafka-1",
            RunnableState::Started,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )],
        Vec::new(),
    );

    context.transition(ArenaLifecycleState::DependenciesStarted);

    let snapshot = recorder.last().expect("a snapshot was published");
    let ids: Vec<&str> = snapshot
        .dependencies
        .iter()
        .map(|dep| dep.id.as_str())
        .collect();
    assert_eq!(ids, vec!["postgres-1", "kafka-1"]);
}

#[test]
fn report_match_one_slot_keeps_other_matches_subjects() {
    let (context, recorder) = context_with_recorder();
    let first = context.reserve_match();
    let second = context.reserve_match();
    context.report_match(
        first,
        vec![DependencyState::new(
            "postgres-1",
            RunnableState::Started,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )],
        Vec::new(),
    );
    context.report_match(
        second,
        vec![DependencyState::new(
            "kafka-1",
            RunnableState::Started,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )],
        Vec::new(),
    );

    context.report_match(
        first,
        vec![DependencyState::new(
            "postgres-1",
            RunnableState::Stopping,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )],
        Vec::new(),
    );
    context.notify_progress();

    let snapshot = recorder.last().expect("a snapshot was published");
    assert_eq!(snapshot.dependencies.len(), 2);
    assert_eq!(snapshot.dependencies[0].state, RunnableState::Stopping);
    assert_eq!(snapshot.dependencies[1].id, "kafka-1");
    assert_eq!(snapshot.dependencies[1].state, RunnableState::Started);
}

#[test]
fn report_match_slot_from_another_publisher_is_refused() {
    let (context, recorder) = context_with_recorder();
    let other = ArenaLifecyclePublisher::new("other-arena", vec![]);
    let unknown = other.reserve_match();

    context.report_match(
        unknown,
        vec![DependencyState::new(
            "postgres-1",
            RunnableState::Started,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )],
        Vec::new(),
    );
    context.transition(ArenaLifecycleState::DependenciesStarted);

    let snapshot = recorder.last().expect("a snapshot was published");
    assert!(snapshot.dependencies.is_empty());
}
