use std::sync::{Arc, Mutex};

use arena::lifecycle::RunnableState;
use arena_profile::{register_observer, unregister_observer, CpuProfileEvent, CpuProfileObserver};

#[derive(Default)]
pub struct RecordedProfiles {
    events: Mutex<Vec<CpuProfileEvent>>,
}

impl CpuProfileObserver for RecordedProfiles {
    fn on_cpu_profile(&self, event: &CpuProfileEvent) {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(event.clone());
    }
}

impl RecordedProfiles {
    pub fn states_for(&self, component_id: &str) -> Vec<RunnableState> {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|event| event.component_id == component_id)
            .map(|event| event.state)
            .collect()
    }

    #[allow(dead_code)]
    pub fn reason_for(&self, component_id: &str) -> Option<String> {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|event| event.component_id == component_id && event.reason.is_some())
            .and_then(|event| event.reason.clone())
    }

    pub fn folded_for(&self, component_id: &str) -> Option<String> {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|event| event.component_id == component_id && event.folded.is_some())
            .and_then(|event| event.folded.clone())
    }
}

pub struct ProfileRecording {
    token: u64,
    pub recorded: Arc<RecordedProfiles>,
}

impl Drop for ProfileRecording {
    fn drop(&mut self) {
        unregister_observer(self.token);
    }
}

pub fn start_recording() -> ProfileRecording {
    let recorded = Arc::new(RecordedProfiles::default());
    let token = register_observer(Arc::clone(&recorded) as Arc<dyn CpuProfileObserver>);
    ProfileRecording { token, recorded }
}
