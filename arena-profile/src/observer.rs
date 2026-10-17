use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, RwLock};

use arena::lifecycle::{panic_message, RunnableState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuProfileEvent {
    pub component_id: String,
    pub state: RunnableState,
    pub folded: Option<String>,
}

impl CpuProfileEvent {
    pub fn new(component_id: impl Into<String>, state: RunnableState) -> Self {
        Self {
            component_id: component_id.into(),
            state,
            folded: None,
        }
    }

    pub fn with_folded(mut self, folded: impl Into<String>) -> Self {
        self.folded = Some(folded.into());
        self
    }
}

pub trait CpuProfileObserver: Send + Sync {
    fn on_cpu_profile(&self, event: &CpuProfileEvent);
}

struct RegisteredObserver {
    token: u64,
    recipient: Arc<dyn CpuProfileObserver>,
}

static REGISTERED_OBSERVERS: LazyLock<RwLock<Vec<RegisteredObserver>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

static NEXT_OBSERVER_TOKEN: AtomicU64 = AtomicU64::new(1);

pub fn register_observer(recipient: Arc<dyn CpuProfileObserver>) -> u64 {
    let token = NEXT_OBSERVER_TOKEN.fetch_add(1, Ordering::Relaxed);
    REGISTERED_OBSERVERS
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .push(RegisteredObserver { token, recipient });
    token
}

pub fn unregister_observer(token: u64) {
    REGISTERED_OBSERVERS
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .retain(|entry| entry.token != token);
}

pub fn notify(event: &CpuProfileEvent) {
    let recipients: Vec<Arc<dyn CpuProfileObserver>> = REGISTERED_OBSERVERS
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .map(|entry| Arc::clone(&entry.recipient))
        .collect();

    for recipient in recipients {
        let outcome = catch_unwind(AssertUnwindSafe(|| recipient.on_cpu_profile(event)));
        if let Err(payload) = outcome {
            tracing::error!(
                component = %event.component_id,
                state = %event.state,
                panic_message = %panic_message(payload.as_ref()),
                phase = "cpu_profile_observer_panic",
                "cpu profile observer panicked; profiling continues"
            );
        }
    }
}

pub fn report(component_id: &str, state: RunnableState) {
    notify(&CpuProfileEvent::new(component_id, state));
}

pub fn report_complete(component_id: &str, folded: &str) {
    notify(&CpuProfileEvent::new(component_id, RunnableState::Stopped).with_folded(folded));
}
