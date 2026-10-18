use crate::lifecycle::state::{ArenaLifecycleState, RunnableState};

pub trait LifecycleRule<S: Copy>: Send + Sync {
    fn is_legal(&self, from: S, to: S) -> bool;
}

pub struct Monotonic;

impl LifecycleRule<ArenaLifecycleState> for Monotonic {
    fn is_legal(&self, from: ArenaLifecycleState, to: ArenaLifecycleState) -> bool {
        to > from
    }
}

pub struct RunnableRule;

impl LifecycleRule<RunnableState> for RunnableRule {
    fn is_legal(&self, from: RunnableState, to: RunnableState) -> bool {
        use RunnableState::*;
        match (from, to) {
            (_, Faulted) => true,
            (NotStarted | Stopped | Faulted, Starting) => true,
            (Starting, ReadinessCheck | Started) => true,
            (ReadinessCheck, Started) => true,
            (Started, Stopping | Stopped) => true,
            (Stopping, Stopped) => true,
            _ => false,
        }
    }
}
