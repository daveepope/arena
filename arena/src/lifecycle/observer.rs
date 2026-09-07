use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

use super::fault::{panic_message, Fault};
use super::snapshot::{ArenaState, ComponentState, DependencyState};
use super::state::ArenaLifecycleState;

pub trait ArenaLifecycleObserver: Send + Sync {
    fn on_state(&self, state: &ArenaState);
}

pub struct LifecycleContext {
    arena_id: String,
    observers: Vec<Arc<dyn ArenaLifecycleObserver>>,
    current: Mutex<ArenaLifecycleState>,
    arena_faults: Mutex<Vec<Fault>>,
}

impl LifecycleContext {
    pub fn new(arena_id: impl Into<String>, observers: Vec<Arc<dyn ArenaLifecycleObserver>>) -> Self {
        Self {
            arena_id: arena_id.into(),
            observers,
            current: Mutex::new(ArenaLifecycleState::ArenaCreated),
            arena_faults: Mutex::new(Vec::new()),
        }
    }

    pub fn arena_id(&self) -> &str {
        &self.arena_id
    }

    pub fn current(&self) -> ArenaLifecycleState {
        *self.current.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn record(&self, fault: Fault) {
        self.arena_faults
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(fault);
    }

    pub fn recorded_faults(&self) -> Vec<Fault> {
        self.arena_faults
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn transition(
        &self,
        state: ArenaLifecycleState,
        dependencies: Vec<DependencyState>,
        components: Vec<ComponentState>,
    ) -> ArenaState {
        let advanced = {
            let mut current = self.current.lock().unwrap_or_else(|e| e.into_inner());
            let advanced = state > *current;
            if advanced {
                *current = state;
            }
            advanced
        };
        let effective = self.current();

        let snapshot = ArenaState::new(
            self.arena_id.clone(),
            effective,
            dependencies,
            components,
            self.recorded_faults(),
        );

        if advanced {
            self.notify(&snapshot);
        }
        snapshot
    }

    pub fn finish(
        &self,
        dependencies: Vec<DependencyState>,
        components: Vec<ComponentState>,
    ) -> ArenaState {
        let pending = ArenaState::new(
            self.arena_id.clone(),
            self.current(),
            dependencies,
            components,
            self.recorded_faults(),
        );
        let terminal = pending.terminal_state();
        self.transition(terminal, pending.dependencies, pending.components)
    }

    fn notify(&self, snapshot: &ArenaState) {
        for observer in &self.observers {
            let outcome = catch_unwind(AssertUnwindSafe(|| observer.on_state(snapshot)));
            if let Err(payload) = outcome {
                tracing::error!(
                    arena = %self.arena_id,
                    state = %snapshot.state,
                    panic_message = %panic_message(payload.as_ref()),
                    phase = "observer_panic",
                    "lifecycle observer panicked; arena lifecycle continues"
                );
            }
        }
    }
}
