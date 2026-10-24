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

    fn flush(&self, deadline: std::time::Instant) {
        self.observer.flush(deadline);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchSlot(usize);

#[derive(Clone, Default)]
struct MatchFrame {
    dependencies: Vec<DependencyState>,
    components: Vec<ComponentState>,
}

pub struct ArenaLifecyclePublisher {
    arena_id: String,
    bus: EventBus<ArenaState>,
    registrations: Mutex<Vec<Registration>>,
    current: Mutex<ArenaLifecycleState>,
    arena_faults: Mutex<Vec<Fault>>,
    frames: Mutex<Vec<MatchFrame>>,
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
            frames: Mutex::new(Vec::new()),
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

    pub fn reserve_match(&self) -> MatchSlot {
        let mut frames = self.frames.lock().unwrap_or_else(|e| e.into_inner());
        frames.push(MatchFrame::default());
        MatchSlot(frames.len() - 1)
    }

    pub fn report_match(
        &self,
        slot: MatchSlot,
        dependencies: Vec<DependencyState>,
        components: Vec<ComponentState>,
    ) {
        let mut frames = self.frames.lock().unwrap_or_else(|e| e.into_inner());
        let Some(frame) = frames.get_mut(slot.0) else {
            tracing::error!(
                arena = %self.arena_id,
                phase = "unknown_match_slot",
                "a match reported against a slot this arena never reserved"
            );
            return;
        };
        *frame = MatchFrame {
            dependencies,
            components,
        };
    }

    fn assembled(&self) -> (Vec<DependencyState>, Vec<ComponentState>) {
        let frames = self.frames.lock().unwrap_or_else(|e| e.into_inner());
        let mut dependencies = Vec::new();
        let mut components = Vec::new();
        for frame in frames.iter() {
            dependencies.extend(frame.dependencies.iter().cloned());
            components.extend(frame.components.iter().cloned());
        }
        (dependencies, components)
    }

    pub fn transition(&self, state: ArenaLifecycleState) -> ArenaState {
        let advanced = {
            let mut current = self.current.lock().unwrap_or_else(|e| e.into_inner());
            let advanced = Monotonic.is_legal(*current, state);
            if advanced {
                *current = state;
            }
            advanced
        };
        let snapshot = self.assemble_snapshot(self.current());

        if advanced {
            self.notify(&snapshot);
        }
        snapshot
    }

    pub fn notify_progress(&self) {
        let snapshot = self.assemble_snapshot(self.current());
        self.notify(&snapshot);
    }

    pub fn finish(&self) -> ArenaState {
        let pending = self.assemble_snapshot(self.current());
        self.transition(pending.terminal_state())
    }

    fn assemble_snapshot(&self, state: ArenaLifecycleState) -> ArenaState {
        let (dependencies, components) = self.assembled();
        ArenaState::new(
            self.arena_id.clone(),
            state,
            dependencies,
            components,
            self.recorded_faults(),
        )
    }

    fn notify(&self, snapshot: &ArenaState) {
        self.bus.publish(snapshot);
    }
}
