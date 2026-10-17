use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use arena::lifecycle::{ArenaLifecycleObserver, ArenaState};

use crate::http_client::build_client;

const INGEST_TIMEOUT: Duration = Duration::from_secs(2);

const DRAIN_BUDGET: Duration = Duration::from_secs(2);

pub struct HttpForwardingObserver {
    sender: Option<Sender<ArenaState>>,
    handle: Option<JoinHandle<()>>,
    drain_deadline: Arc<OnceLock<Instant>>,
}

impl HttpForwardingObserver {
    pub fn new(dashboard_url: impl Into<String>) -> Self {
        let dashboard_url = dashboard_url.into();
        let (sender, receiver) = channel::<ArenaState>();
        let drain_deadline = Arc::new(OnceLock::new());
        let forwarding_deadline = Arc::clone(&drain_deadline);
        let handle = thread::spawn(move || {
            let Some(client) = build_client(INGEST_TIMEOUT) else {
                for _ in receiver {}
                return;
            };
            let ingest_url = format!("{}/ingest", dashboard_url.trim_end_matches('/'));
            let mut past_budget = 0usize;
            for state in receiver {
                if forwarding_deadline
                    .get()
                    .is_some_and(|deadline| Instant::now() >= *deadline)
                {
                    past_budget += 1;
                    continue;
                }
                let _ = client.post(&ingest_url).json(&state).send();
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
        if let Some(sender) = &self.sender {
            if sender.send(state.clone()).is_err() {
                tracing::warn!("arena-dashboard forwarding thread is gone, dropping state update");
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
