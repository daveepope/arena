#[path = "../subjects.rs"]
mod subjects;

use arena::lifecycle::{
    ArenaLifecycleState, ComponentState, DependencyState, Fault, LifecycleContext, RunnableState,
};
use std::sync::Arc;
use subjects::StateRecorder;

fn context_with_recorder() -> (LifecycleContext, Arc<StateRecorder>) {
    let recorder = Arc::new(StateRecorder::default());
    let context = LifecycleContext::new("test-arena", vec![recorder.clone()]);
    (context, recorder)
}

fn no_subjects() -> (Vec<DependencyState>, Vec<ComponentState>) {
    (Vec::new(), Vec::new())
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
        let (deps, comps) = no_subjects();
        context.transition(state, deps, comps);
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

    let (deps, comps) = no_subjects();
    context.transition(ArenaLifecycleState::ComponentsStarted, deps, comps);
    let (deps, comps) = no_subjects();
    let snapshot = context.transition(ArenaLifecycleState::DependenciesStarting, deps, comps);

    assert_eq!(context.current(), ArenaLifecycleState::ComponentsStarted);
    assert_eq!(snapshot.state, ArenaLifecycleState::ComponentsStarted);
    assert_eq!(recorder.states(), vec![ArenaLifecycleState::ComponentsStarted]);
}

#[test]
fn transition_repeated_state_notifies_only_once() {
    let (context, recorder) = context_with_recorder();

    for _ in 0..3 {
        let (deps, comps) = no_subjects();
        context.transition(ArenaLifecycleState::ArenaStarting, deps, comps);
    }

    assert_eq!(recorder.states(), vec![ArenaLifecycleState::ArenaStarting]);
}

#[test]
fn record_arena_fault_appears_in_next_snapshot() {
    let (context, _recorder) = context_with_recorder();
    context.record(Fault::arena("test-arena", "runtime unavailable"));

    let (deps, comps) = no_subjects();
    let snapshot = context.transition(ArenaLifecycleState::ArenaStarting, deps, comps);

    assert_eq!(snapshot.faults.len(), 1);
    assert_eq!(snapshot.faults[0].message, "runtime unavailable");
    assert_eq!(context.recorded_faults().len(), 1);
}

#[test]
fn finish_clean_subjects_returns_arena_closed() {
    let (context, recorder) = context_with_recorder();
    let (deps, comps) = no_subjects();
    context.transition(ArenaLifecycleState::ArenaTeardown, deps, comps);

    let state = context.finish(
        vec![DependencyState::new(
            "postgres-1",
            RunnableState::Stopped,
            Vec::new(),
            Vec::new(),
        )],
        Vec::new(),
    );

    assert_eq!(state.state, ArenaLifecycleState::ArenaClosed);
    assert_eq!(recorder.states().last(), Some(&ArenaLifecycleState::ArenaClosed));
}

#[test]
fn finish_faulted_subject_returns_arena_faulted() {
    let (context, recorder) = context_with_recorder();
    let (deps, comps) = no_subjects();
    context.transition(ArenaLifecycleState::ArenaTeardown, deps, comps);

    let state = context.finish(
        Vec::new(),
        vec![ComponentState::new(
            "api",
            RunnableState::Faulted,
            vec![Fault::component("api", "could not confirm removal")],
            Vec::new(),
        )],
    );

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert_eq!(recorder.states().last(), Some(&ArenaLifecycleState::ArenaFaulted));
}

#[test]
fn finish_recorded_fault_only_returns_arena_faulted() {
    let (context, _recorder) = context_with_recorder();
    context.record(Fault::playbook("seed", "seed data rejected"));

    let state = context.finish(Vec::new(), Vec::new());

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
}

#[test]
fn arena_id_any_context_returns_the_configured_id() {
    let (context, _recorder) = context_with_recorder();

    assert_eq!(context.arena_id(), "test-arena");
}
