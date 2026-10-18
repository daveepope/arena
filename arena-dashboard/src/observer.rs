use std::sync::mpsc::{channel, Sender};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use arena::lifecycle::bus::Observer;
use arena::lifecycle::{ArenaLifecycleObserver, ArenaState};
use arena::playbook::ScopedPlaybookEvent;
use arena_profile::{CpuProfileEvent, CpuProfileObserver};

use crate::http_client::build_client;

const INGEST_TIMEOUT: Duration = Duration::from_secs(2);

const DRAIN_BUDGET: Duration = Duration::from_secs(2);

enum Forwarded {
    State(ArenaState),
    CpuProfile(CpuProfileEvent),
    ScopedPlaybook(ScopedPlaybookEvent),
}

const DRAIN_POLL_INTERVAL: Duration = Duration::from_millis(5);

pub struct HttpForwardingObserver {
    sender: Mutex<Option<Sender<Forwarded>>>,
    handle: Mutex<Option<JoinHandle<()>>>,
    drain_deadline: Arc<OnceLock<Instant>>,
    pending: Arc<AtomicUsize>,
}

impl HttpForwardingObserver {
    pub fn new(dashboard_url: impl Into<String>) -> Self {
        let dashboard_url = dashboard_url.into();
        let (sender, receiver) = channel::<Forwarded>();
        let drain_deadline = Arc::new(OnceLock::new());
        let forwarding_deadline = Arc::clone(&drain_deadline);
        let pending = Arc::new(AtomicUsize::new(0));
        let forwarding_pending = Arc::clone(&pending);
        let handle = thread::spawn(move || {
            let Some(client) = build_client(INGEST_TIMEOUT) else {
                for _ in receiver {}
                return;
            };
            let base_url = dashboard_url.trim_end_matches('/');
            let ingest_url = format!("{base_url}/ingest");
            let cpu_profile_url = format!("{base_url}/profile/cpu");
            let scoped_playbook_url = format!("{base_url}/playbook/scoped");
            let mut past_budget = 0usize;
            for forwarded in receiver {
                let _outstanding = PendingGuard(Arc::clone(&forwarding_pending));
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
                    Forwarded::ScopedPlaybook(event) => {
                        let _ = client.post(&scoped_playbook_url).json(&event).send();
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
            sender: Mutex::new(Some(sender)),
            handle: Mutex::new(Some(handle)),
            drain_deadline,
            pending,
        }
    }
}

impl ArenaLifecycleObserver for HttpForwardingObserver {
    fn on_state(&self, state: &ArenaState) {
        self.forward(Forwarded::State(state.clone()), "state update");
    }

    fn flush(&self, deadline: Instant) {
        self.flush_within(deadline.saturating_duration_since(Instant::now()));
    }
}

impl CpuProfileObserver for HttpForwardingObserver {
    fn on_cpu_profile(&self, event: &CpuProfileEvent) {
        self.forward(Forwarded::CpuProfile(event.clone()), "cpu profile update");
    }
}

pub struct ScopedPlaybookForwarder {
    forwarder: Arc<HttpForwardingObserver>,
}

impl ScopedPlaybookForwarder {
    pub fn new(forwarder: Arc<HttpForwardingObserver>) -> Arc<Self> {
        Arc::new(Self { forwarder })
    }
}

impl Observer<ScopedPlaybookEvent> for ScopedPlaybookForwarder {
    fn on_event(&self, event: &ScopedPlaybookEvent) {
        self.forwarder.forward(
            Forwarded::ScopedPlaybook(event.clone()),
            "scoped playbook update",
        );
    }

    fn flush(&self, deadline: Instant) {
        self.forwarder
            .flush_within(deadline.saturating_duration_since(Instant::now()));
    }
}

impl HttpForwardingObserver {
    fn forward(&self, forwarded: Forwarded, what: &str) {
        let sender = self.sender.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(sender) = sender.as_ref() {
            self.pending.fetch_add(1, Ordering::SeqCst);
            if sender.send(forwarded).is_err() {
                self.pending.fetch_sub(1, Ordering::SeqCst);
                tracing::warn!(
                    dropped = what,
                    "arena-dashboard forwarding thread is gone, dropping update"
                );
            }
        }
    }
}

struct PendingGuard(Arc<AtomicUsize>);

impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl HttpForwardingObserver {
    pub fn flush_within(&self, budget: Duration) {
        let deadline = Instant::now() + budget;
        while self.pending.load(Ordering::SeqCst) > 0 {
            if Instant::now() >= deadline {
                tracing::warn!(
                    outstanding = self.pending.load(Ordering::SeqCst),
                    budget = ?budget,
                    "arena-dashboard flush budget reached with updates still in flight"
                );
                return;
            }
            std::thread::sleep(DRAIN_POLL_INTERVAL);
        }
    }

    fn shut_down(&self, budget: Duration) {
        let _ = self.drain_deadline.set(Instant::now() + budget);
        self.sender.lock().unwrap_or_else(|e| e.into_inner()).take();
        let handle = self.handle.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}

impl Drop for HttpForwardingObserver {
    fn drop(&mut self) {
        self.shut_down(DRAIN_BUDGET);
    }
}
