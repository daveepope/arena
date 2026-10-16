use std::sync::mpsc::{channel, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use arena::lifecycle::{ArenaLifecycleObserver, ArenaState};

const INGEST_TIMEOUT: Duration = Duration::from_secs(2);

pub struct HttpForwardingObserver {
    sender: Option<Sender<ArenaState>>,
    handle: Option<JoinHandle<()>>,
}

impl HttpForwardingObserver {
    pub fn new(dashboard_url: impl Into<String>) -> Self {
        let dashboard_url = dashboard_url.into();
        let (sender, receiver) = channel::<ArenaState>();
        let handle = thread::spawn(move || {
            let client = reqwest::blocking::Client::new();
            let ingest_url = format!("{}/ingest", dashboard_url.trim_end_matches('/'));
            for state in receiver {
                let _ = client.post(&ingest_url).timeout(INGEST_TIMEOUT).json(&state).send();
            }
        });
        Self { sender: Some(sender), handle: Some(handle) }
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
        self.sender.take();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
