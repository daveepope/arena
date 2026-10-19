use std::sync::mpsc::{channel, Sender};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use arena::lifecycle::bus::Observer;
use arena::lifecycle::{ArenaLifecycleObserver, ArenaState};
use arena::playbook::PlaybookEvent;
use arena_profile::{CpuProfileEvent, CpuProfileObserver};

use crate::http_client::build_client;

const INGEST_TIMEOUT: Duration = Duration::from_secs(2);

const DRAIN_BUDGET: Duration = Duration::from_secs(2);

enum Forwarded {
    State(ArenaState),
    CpuProfile(CpuProfileEvent),
    Playbook(PlaybookEvent),
}

pub struct HttpForwardingObserver {
    sender: Mutex<Option<Sender<Forwarded>>>,
    handle: Mutex<Option<JoinHandle<()>>>,
    drain_deadline: Arc<OnceLock<Instant>>,
    pending: Arc<AtomicUsize>,
    drained: Arc<(Mutex<()>, Condvar)>,
}

impl HttpForwardingObserver {
    pub fn new(dashboard_url: impl Into<String>) -> Self {
        let dashboard_url = dashboard_url.into();
        let (sender, receiver) = channel::<Forwarded>();
        let drain_deadline = Arc::new(OnceLock::new());
        let forwarding_deadline = Arc::clone(&drain_deadline);
        let pending = Arc::new(AtomicUsize::new(0));
        let forwarding_pending = Arc::clone(&pending);
        let drained: Arc<(Mutex<()>, Condvar)> = Arc::new((Mutex::new(()), Condvar::new()));
        let forwarding_drained = Arc::clone(&drained);

        let handle = thread::spawn(move || {
            let Some(client) = build_client(INGEST_TIMEOUT) else {
                for _ in receiver {
                    forwarding_pending.fetch_sub(1, Ordering::SeqCst);
                    forwarding_drained.1.notify_all();
                }
                return;
            };
            let base_url = dashboard_url.trim_end_matches('/');
            let ingest_url = format!("{base_url}/ingest");
            let cpu_profile_url = format!("{base_url}/profile/cpu");
            let playbook_base = base_url.to_string();
            let mut past_budget = 0usize;
            for forwarded in receiver {
                let _outstanding =
                    PendingGuard(Arc::clone(&forwarding_pending), Arc::clone(&forwarding_drained));
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
                    Forwarded::Playbook(event) => {
                        let Some(url) = playbook_url(
                            &playbook_base,
                            &event.arena_id,
                            &event.id,
                        ) else {
                            continue;
                        };
                        match client.post(url).json(&event).send() {
                            Ok(response) if !response.status().is_success() => {
                                tracing::warn!(
                                    status = %response.status(),
                                    playbook = %event.id,
                                    "arena-dashboard rejected a playbook update"
                                );
                            }
                            _ => {}
                        }
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
            drained,
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

pub struct PlaybookForwarder {
    forwarder: Arc<HttpForwardingObserver>,
}

impl PlaybookForwarder {
    pub fn new(forwarder: Arc<HttpForwardingObserver>) -> Arc<Self> {
        Arc::new(Self { forwarder })
    }
}

impl Observer<PlaybookEvent> for PlaybookForwarder {
    fn on_event(&self, event: &PlaybookEvent) {
        self.forwarder.forward(
            Forwarded::Playbook(event.clone()),
            "playbook update",
        );
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

fn playbook_url(base: &str, arena_id: &str, playbook_id: &str) -> Option<reqwest::Url> {
    let mut url = reqwest::Url::parse(base).ok()?;
    url.path_segments_mut()
        .ok()?
        .pop_if_empty()
        .extend(["arena", arena_id, "playbook", playbook_id]);
    Some(url)
}

struct PendingGuard(Arc<AtomicUsize>, Arc<(Mutex<()>, Condvar)>);

impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
        self.1 .1.notify_all();
    }
}

impl HttpForwardingObserver {
    pub fn flush_within(&self, budget: Duration) {
        let deadline = Instant::now() + budget;
        let (lock, drained) = &*self.drained;
        let mut waiting = lock.lock().unwrap_or_else(|e| e.into_inner());
        while self.pending.load(Ordering::SeqCst) > 0 {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                tracing::warn!(
                    outstanding = self.pending.load(Ordering::SeqCst),
                    budget = ?budget,
                    "arena-dashboard flush budget reached with updates still in flight"
                );
                return;
            };
            let (next, outcome) = drained
                .wait_timeout(waiting, remaining)
                .unwrap_or_else(|e| e.into_inner());
            waiting = next;
            if outcome.timed_out() && self.pending.load(Ordering::SeqCst) > 0 {
                tracing::warn!(
                    outstanding = self.pending.load(Ordering::SeqCst),
                    budget = ?budget,
                    "arena-dashboard flush budget reached with updates still in flight"
                );
                return;
            }
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
