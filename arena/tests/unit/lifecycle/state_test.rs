use arena::lifecycle::{ArenaLifecycleState, RunnableState};
use serde_json::json;

#[test]
fn arena_lifecycle_state_full_sequence_is_strictly_increasing() {
    let sequence = [
        ArenaLifecycleState::ArenaCreated,
        ArenaLifecycleState::ArenaStarting,
        ArenaLifecycleState::DependenciesStarting,
        ArenaLifecycleState::DependenciesStarted,
        ArenaLifecycleState::PlaybooksRunning,
        ArenaLifecycleState::PlaybooksComplete,
        ArenaLifecycleState::ComponentsStarting,
        ArenaLifecycleState::ComponentsStarted,
        ArenaLifecycleState::ArenaOpen,
        ArenaLifecycleState::ArenaClosing,
        ArenaLifecycleState::ComponentsStopping,
        ArenaLifecycleState::ComponentsStopped,
        ArenaLifecycleState::DependenciesStopping,
        ArenaLifecycleState::DependenciesStopped,
        ArenaLifecycleState::ArenaTeardown,
        ArenaLifecycleState::ArenaClosed,
        ArenaLifecycleState::ArenaFaulted,
    ];

    for pair in sequence.windows(2) {
        assert!(pair[0] < pair[1], "{:?} should precede {:?}", pair[0], pair[1]);
    }
}

#[test]
fn is_final_terminal_states_returns_true() {
    assert!(ArenaLifecycleState::ArenaClosed.is_final());
    assert!(ArenaLifecycleState::ArenaFaulted.is_final());
    assert!(!ArenaLifecycleState::ArenaOpen.is_final());
    assert!(!ArenaLifecycleState::ArenaTeardown.is_final());
}

#[test]
fn as_str_every_arena_state_returns_snake_case_name() {
    assert_eq!(ArenaLifecycleState::ArenaCreated.as_str(), "arena_created");
    assert_eq!(
        ArenaLifecycleState::DependenciesStarting.as_str(),
        "dependencies_starting"
    );
    assert_eq!(ArenaLifecycleState::ArenaTeardown.as_str(), "arena_teardown");
    assert_eq!(ArenaLifecycleState::ArenaFaulted.as_str(), "arena_faulted");
    assert_eq!(
        ArenaLifecycleState::ArenaOpen.to_string(),
        ArenaLifecycleState::ArenaOpen.as_str()
    );
}

#[test]
fn runnable_state_default_returns_not_started() {
    assert_eq!(RunnableState::default(), RunnableState::NotStarted);
}

#[test]
fn is_final_stopped_and_faulted_returns_true() {
    assert!(RunnableState::Stopped.is_final());
    assert!(RunnableState::Faulted.is_final());
    assert!(!RunnableState::Started.is_final());
    assert!(!RunnableState::ReadinessCheck.is_final());
}

#[test]
fn is_inactive_not_started_and_stopped_returns_true() {
    assert!(RunnableState::NotStarted.is_inactive());
    assert!(RunnableState::Stopped.is_inactive());
    assert!(!RunnableState::Faulted.is_inactive());
    assert!(!RunnableState::Starting.is_inactive());
    assert!(!RunnableState::Started.is_inactive());
}

#[test]
fn as_str_every_runnable_state_returns_snake_case_name() {
    assert_eq!(RunnableState::NotStarted.as_str(), "not_started");
    assert_eq!(RunnableState::ReadinessCheck.as_str(), "readiness_check");
    assert_eq!(RunnableState::Faulted.as_str(), "faulted");
    assert_eq!(
        RunnableState::Stopping.to_string(),
        RunnableState::Stopping.as_str()
    );
}

#[test]
fn as_str_arena_states_returns_unique_snake_case_names() {
    let states = [
        ArenaLifecycleState::ArenaCreated,
        ArenaLifecycleState::ArenaStarting,
        ArenaLifecycleState::DependenciesStarting,
        ArenaLifecycleState::DependenciesStarted,
        ArenaLifecycleState::PlaybooksRunning,
        ArenaLifecycleState::PlaybooksComplete,
        ArenaLifecycleState::ComponentsStarting,
        ArenaLifecycleState::ComponentsStarted,
        ArenaLifecycleState::ArenaOpen,
        ArenaLifecycleState::ArenaClosing,
        ArenaLifecycleState::ComponentsStopping,
        ArenaLifecycleState::ComponentsStopped,
        ArenaLifecycleState::DependenciesStopping,
        ArenaLifecycleState::DependenciesStopped,
        ArenaLifecycleState::ArenaTeardown,
        ArenaLifecycleState::ArenaClosed,
        ArenaLifecycleState::ArenaFaulted,
    ];

    let names: Vec<&str> = states.iter().map(|s| s.as_str()).collect();
    let mut unique = names.clone();
    unique.sort_unstable();
    unique.dedup();

    assert_eq!(unique.len(), states.len(), "every state needs a distinct name");
    for (state, name) in states.iter().zip(names.iter()) {
        assert_eq!(&state.to_string(), name);
        assert!(
            name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "{name} is not snake_case"
        );
    }
}

#[test]
fn as_str_runnable_states_returns_unique_snake_case_names() {
    let states = [
        RunnableState::NotStarted,
        RunnableState::Starting,
        RunnableState::ReadinessCheck,
        RunnableState::Started,
        RunnableState::Stopping,
        RunnableState::Stopped,
        RunnableState::Faulted,
    ];

    let names: Vec<&str> = states.iter().map(|s| s.as_str()).collect();
    let mut unique = names.clone();
    unique.sort_unstable();
    unique.dedup();

    assert_eq!(unique.len(), states.len(), "every state needs a distinct name");
    for (state, name) in states.iter().zip(names.iter()) {
        assert_eq!(&state.to_string(), name);
        assert!(
            name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "{name} is not snake_case"
        );
    }
}

#[test]
fn serialize_every_arena_state_returns_its_as_str_token() {
    let states = [
        ArenaLifecycleState::ArenaCreated,
        ArenaLifecycleState::ArenaStarting,
        ArenaLifecycleState::DependenciesStarting,
        ArenaLifecycleState::DependenciesStarted,
        ArenaLifecycleState::PlaybooksRunning,
        ArenaLifecycleState::PlaybooksComplete,
        ArenaLifecycleState::ComponentsStarting,
        ArenaLifecycleState::ComponentsStarted,
        ArenaLifecycleState::ArenaOpen,
        ArenaLifecycleState::ArenaClosing,
        ArenaLifecycleState::ComponentsStopping,
        ArenaLifecycleState::ComponentsStopped,
        ArenaLifecycleState::DependenciesStopping,
        ArenaLifecycleState::DependenciesStopped,
        ArenaLifecycleState::ArenaTeardown,
        ArenaLifecycleState::ArenaClosed,
        ArenaLifecycleState::ArenaFaulted,
    ];

    for state in states {
        assert_eq!(serde_json::to_value(state).unwrap(), json!(state.as_str()));
    }
}

#[test]
fn serialize_every_runnable_state_returns_its_as_str_token() {
    let states = [
        RunnableState::NotStarted,
        RunnableState::Starting,
        RunnableState::ReadinessCheck,
        RunnableState::Started,
        RunnableState::Stopping,
        RunnableState::Stopped,
        RunnableState::Faulted,
    ];

    for state in states {
        assert_eq!(serde_json::to_value(state).unwrap(), json!(state.as_str()));
    }
}
