use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::bus::{EventBus, Observer, Registration};
use super::fault::Fault;
use super::observer::ArenaLifecycleObserver;
use super::rule::{LifecycleRule, Monotonic};
use super::snapshot::{ArenaState, ComponentState, DependencyState};
use super::state::ArenaLifecycleState;

const CHANNEL: &str = "arena-lifecycle";

struct ObserverAdapter {
    observer: Arc<dyn ArenaLifecycleObserver>,
}

impl Observer<ArenaState> for ObserverAdapter {
    fn on_event(&self, event: &ArenaState) {
        self.observer.on_state(event);
    }
}

pub struct ArenaLifecyclePublisher {
    arena_id: String,
    bus: EventBus<ArenaState>,
    registrations: Mutex<Vec<Registration>>,
    current: Mutex<ArenaLifecycleState>,
    arena_faults: Mutex<Vec<Fault>>,
}

impl ArenaLifecyclePublisher {
    pub fn new(arena_id: impl Into<String>, observers: Vec<Arc<dyn ArenaLifecycleObserver>>) -> Self {
        let bus = EventBus::new(CHANNEL);
        let registrations = observers
            .into_iter()
            .map(|observer| bus.register_observer(Arc::new(ObserverAdapter { observer })))
            .collect();
        Self {
            arena_id: arena_id.into(),
            bus,
            registrations: Mutex::new(registrations),
            current: Mutex::new(ArenaLifecycleState::ArenaCreated),
            arena_faults: Mutex::new(Vec::new()),
        }
    }

    pub fn flush(&self, budget: Duration) {
        self.bus.flush(budget);
    }

    pub fn unregister_all(&self) {
        for registration in self
            .registrations
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain(..)
        {
            self.bus.unregister_observer(registration);
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
            let advanced = Monotonic.is_legal(*current, state);
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

    pub fn notify_progress(&self, dependencies: Vec<DependencyState>, components: Vec<ComponentState>) {
        let snapshot = ArenaState::new(
            self.arena_id.clone(),
            self.current(),
            dependencies,
            components,
            self.recorded_faults(),
        );
        self.notify(&snapshot);
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
        self.bus.publish(snapshot);
    }
}
