use std::sync::{Arc, Mutex};

use crate::lifecycle::bus::Publisher;
use crate::lifecycle::fault::Fault;
use crate::lifecycle::rule::{LifecycleRule, RunnableRule};
use crate::lifecycle::state::RunnableState;

pub const CHANNEL: &str = "scoped-playbook";

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ScopedPlaybookEvent {
    pub arena_id: String,
    pub playbook_id: String,
    pub state: RunnableState,
    pub faults: Vec<Fault>,
}

struct Attachment {
    arena_id: String,
    publisher: Arc<dyn Publisher<ScopedPlaybookEvent>>,
}

pub struct ScopedPlaybookLifecycle {
    playbook_id: String,
    state: Mutex<RunnableState>,
    faults: Mutex<Vec<Fault>>,
    attachment: Mutex<Option<Attachment>>,
}

impl ScopedPlaybookLifecycle {
    pub fn new(playbook_id: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            playbook_id: playbook_id.into(),
            state: Mutex::new(RunnableState::NotStarted),
            faults: Mutex::new(Vec::new()),
            attachment: Mutex::new(None),
        })
    }

    pub fn attach(
        &self,
        arena_id: impl Into<String>,
        publisher: Arc<dyn Publisher<ScopedPlaybookEvent>>,
    ) {
        *self.attachment.lock().unwrap_or_else(|e| e.into_inner()) = Some(Attachment {
            arena_id: arena_id.into(),
            publisher,
        });
    }

    pub fn playbook_id(&self) -> &str {
        &self.playbook_id
    }

    pub fn state(&self) -> RunnableState {
        *self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn faults(&self) -> Vec<Fault> {
        self.faults.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn report(&self, next: RunnableState) {
        {
            let mut current = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if !RunnableRule.is_legal(*current, next) {
                tracing::error!(
                    playbook = %self.playbook_id,
                    from = %*current,
                    to = %next,
                    phase = "illegal_playbook_transition",
                    "refusing to report an illegal playbook transition"
                );
                return;
            }
            *current = next;
        }
        self.publish(next);
    }

    pub fn report_terminal(&self) {
        if self.state() != RunnableState::Started {
            return;
        }
        self.report(RunnableState::Stopping);
        self.report(RunnableState::Stopped);
    }

    pub fn report_fault(&self, fault: Fault) {
        self.faults
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(fault);
        self.report(RunnableState::Faulted);
    }

    fn publish(&self, state: RunnableState) {
        let attachment = self.attachment.lock().unwrap_or_else(|e| e.into_inner());
        let Some(attachment) = attachment.as_ref() else {
            return;
        };
        attachment.publisher.publish(&ScopedPlaybookEvent {
            arena_id: attachment.arena_id.clone(),
            playbook_id: self.playbook_id.clone(),
            state,
            faults: self.faults(),
        });
    }
}
