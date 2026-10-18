use super::snapshot::ArenaState;

pub trait ArenaLifecycleObserver: Send + Sync {
    fn on_state(&self, state: &ArenaState);
}
