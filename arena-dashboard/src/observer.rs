use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use arena::lifecycle::{ArenaLifecycleObserver, ArenaState};
use arena_profile::{CpuProfileEvent, CpuProfileObserver};

use crate::http_client::build_client;

const INGEST_TIMEOUT: Duration = Duration::from_secs(2);

const DRAIN_BUDGET: Duration = Duration::from_secs(2);

enum Forwarded {
    State(ArenaState),
    CpuProfile(CpuProfileEvent),
}

pub struct HttpForwardingObserver {
    sender: Option<Sender<Forwarded>>,
    handle: Option<JoinHandle<()>>,
    drain_deadline: Arc<OnceLock<Instant>>,
}

impl HttpForwardingObserver {
    pub fn new(dashboard_url: impl Into<String>) -> Self {
        let dashboard_url = dashboard_url.into();
        let (sender, receiver) = channel::<Forwarded>();
        let drain_deadline = Arc::new(OnceLock::new());
        let forwarding_deadline = Arc::clone(&drain_deadline);
        let handle = thread::spawn(move || {
            let Some(client) = build_client(INGEST_TIMEOUT) else {
                for _ in receiver {}
                return;
            };
            let base_url = dashboard_url.trim_end_matches('/');
            let ingest_url = format!("{base_url}/ingest");
            let cpu_profile_url = format!("{base_url}/profile/cpu");
            let mut past_budget = 0usize;
            for forwarded in receiver {
                if forwarding_deadline
                    .get()
                    .is_some_and(|deadline| Instant::now() >= *deadline)
                {
                    past_budget += 1;
                    continue;
                }
                match forwarded {
                    Forwarded::State(state) => {
                        let _ = client.post(&ingest_url).json(&state).send();
                    }
                    Forwarded::CpuProfile(event) => {
                        let payload = crate::ingest::CpuProfilePayload {
                            component: event.component_id,
                            state: event.state.as_str().to_string(),
                            folded: event.folded,
                        };
                        let _ = client.post(&cpu_profile_url).json(&payload).send();
                    }
                }
            }
            if past_budget > 0 {
                tracing::warn!(
                    not_forwarded = past_budget,
                    budget = ?DRAIN_BUDGET,
                    "arena-dashboard teardown budget reached, trailing state updates were not forwarded"
                );
            }
        });
        Self {
            sender: Some(sender),
            handle: Some(handle),
            drain_deadline,
        }
    }
}

impl ArenaLifecycleObserver for HttpForwardingObserver {
    fn on_state(&self, state: &ArenaState) {
        self.forward(Forwarded::State(state.clone()), "state update");
    }
}

impl CpuProfileObserver for HttpForwardingObserver {
    fn on_cpu_profile(&self, event: &CpuProfileEvent) {
        self.forward(Forwarded::CpuProfile(event.clone()), "cpu profile update");
    }
}

impl HttpForwardingObserver {
    fn forward(&self, forwarded: Forwarded, what: &str) {
        if let Some(sender) = &self.sender {
            if sender.send(forwarded).is_err() {
                tracing::warn!(
                    dropped = what,
                    "arena-dashboard forwarding thread is gone, dropping update"
                );
            }
        }
    }
}

impl Drop for HttpForwardingObserver {
    fn drop(&mut self) {
        let _ = self.drain_deadline.set(Instant::now() + DRAIN_BUDGET);
        self.sender.take();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
