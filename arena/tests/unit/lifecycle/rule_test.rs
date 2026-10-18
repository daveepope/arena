use arena::lifecycle::rule::{LifecycleRule, Monotonic, RunnableRule};
use arena::lifecycle::state::{ArenaLifecycleState, RunnableState};

const EVERY_RUNNABLE_STATE: [RunnableState; 7] = [
    RunnableState::NotStarted,
    RunnableState::Starting,
    RunnableState::ReadinessCheck,
    RunnableState::Started,
    RunnableState::Stopping,
    RunnableState::Stopped,
    RunnableState::Faulted,
];

#[test]
fn is_legal_forward_through_a_run_is_accepted() {
    let run = [
        (RunnableState::NotStarted, RunnableState::Starting),
        (RunnableState::Starting, RunnableState::ReadinessCheck),
        (RunnableState::ReadinessCheck, RunnableState::Started),
        (RunnableState::Started, RunnableState::Stopping),
        (RunnableState::Stopping, RunnableState::Stopped),
    ];

    for (from, to) in run {
        assert!(RunnableRule.is_legal(from, to), "{from} -> {to}");
    }
}

#[test]
fn is_legal_run_skipping_readiness_check_is_accepted() {
    assert!(RunnableRule.is_legal(RunnableState::Starting, RunnableState::Started));
}

#[test]
fn is_legal_stop_without_stopping_is_accepted() {
    assert!(RunnableRule.is_legal(RunnableState::Started, RunnableState::Stopped));
}

#[test]
fn is_legal_fault_from_every_state_is_accepted() {
    for from in EVERY_RUNNABLE_STATE {
        assert!(
            RunnableRule.is_legal(from, RunnableState::Faulted),
            "{from} -> faulted"
        );
    }
}

#[test]
fn is_legal_restart_from_a_terminal_state_is_accepted() {
    assert!(RunnableRule.is_legal(RunnableState::Stopped, RunnableState::Starting));
    assert!(RunnableRule.is_legal(RunnableState::Faulted, RunnableState::Starting));
}

#[test]
fn is_legal_backwards_within_a_run_is_rejected() {
    let backwards = [
        (RunnableState::Started, RunnableState::Starting),
        (RunnableState::Started, RunnableState::NotStarted),
        (RunnableState::Stopping, RunnableState::Started),
        (RunnableState::ReadinessCheck, RunnableState::Starting),
    ];

    for (from, to) in backwards {
        assert!(!RunnableRule.is_legal(from, to), "{from} -> {to}");
    }
}

#[test]
fn is_legal_skipping_forward_is_rejected() {
    let skipping = [
        (RunnableState::NotStarted, RunnableState::Started),
        (RunnableState::NotStarted, RunnableState::Stopped),
        (RunnableState::Stopped, RunnableState::Started),
        (RunnableState::Starting, RunnableState::Stopped),
    ];

    for (from, to) in skipping {
        assert!(!RunnableRule.is_legal(from, to), "{from} -> {to}");
    }
}

#[test]
fn is_legal_same_state_twice_is_rejected() {
    for state in EVERY_RUNNABLE_STATE {
        if state == RunnableState::Faulted {
            continue;
        }
        assert!(!RunnableRule.is_legal(state, state), "{state} -> {state}");
    }
}

#[test]
fn is_legal_arena_phase_moving_forward_is_accepted() {
    assert!(Monotonic.is_legal(
        ArenaLifecycleState::ArenaCreated,
        ArenaLifecycleState::ArenaStarting
    ));
    assert!(Monotonic.is_legal(
        ArenaLifecycleState::PlaybooksRunning,
        ArenaLifecycleState::PlaybooksComplete
    ));
}

#[test]
fn is_legal_arena_phase_moving_backwards_or_repeating_is_rejected() {
    assert!(!Monotonic.is_legal(
        ArenaLifecycleState::ArenaOpen,
        ArenaLifecycleState::ArenaStarting
    ));
    assert!(!Monotonic.is_legal(
        ArenaLifecycleState::ArenaOpen,
        ArenaLifecycleState::ArenaOpen
    ));
}
