use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::lifecycle::bus::Publisher;
use crate::lifecycle::fault::{serialize_timestamp, Fault};
use crate::lifecycle::rule::LifecycleRule;
use crate::playbook::state::PlaybookRule;
use crate::playbook::state::PlaybookState;

pub const CHANNEL: &str = "playbook";

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybookType {
    Managed,
    Unmanaged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybookKind {
    Startup,
    Scoped,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PlaybookEvent {
    pub arena_id: String,
    pub id: String,
    pub playbook_type: PlaybookType,
    pub kind: PlaybookKind,
    pub state: PlaybookState,
    pub runs: usize,
    pub faults: Vec<Fault>,
    #[serde(serialize_with = "serialize_timestamp")]
    pub at: DateTime<Utc>,
}

impl PlaybookEvent {
    pub fn new(
        arena_id: impl Into<String>,
        id: impl Into<String>,
        playbook_type: PlaybookType,
        kind: PlaybookKind,
        state: PlaybookState,
        runs: usize,
        faults: Vec<Fault>,
    ) -> Self {
        Self {
            arena_id: arena_id.into(),
            id: id.into(),
            playbook_type,
            kind,
            state,
            runs,
            faults,
            at: Utc::now(),
        }
    }
}

struct Attachment {
    arena_id: String,
    kind: PlaybookKind,
    publisher: Arc<dyn Publisher<PlaybookEvent>>,
}

#[derive(Clone)]
struct Progress {
    state: PlaybookState,
    runs: usize,
    faults: Vec<Fault>,
}

impl Progress {
    fn fresh() -> Self {
        Self {
            state: PlaybookState::Registered,
            runs: 0,
            faults: Vec::new(),
        }
    }
}

pub struct PlaybookLifecycle {
    playbook_id: String,
    playbook_type: PlaybookType,
    progress: Mutex<Progress>,
    attachment: Mutex<Option<Attachment>>,
    live_handles: AtomicUsize,
}

impl PlaybookLifecycle {
    pub fn new(playbook_id: impl Into<String>) -> Arc<Self> {
        Self::of_type(playbook_id, PlaybookType::Unmanaged)
    }

    pub fn managed(playbook_id: impl Into<String>) -> Arc<Self> {
        Self::of_type(playbook_id, PlaybookType::Managed)
    }

    fn of_type(playbook_id: impl Into<String>, playbook_type: PlaybookType) -> Arc<Self> {
        Arc::new(Self {
            playbook_id: playbook_id.into(),
            playbook_type,
            progress: Mutex::new(Progress::fresh()),
            attachment: Mutex::new(None),
            live_handles: AtomicUsize::new(0),
        })
    }

    pub fn attach(
        &self,
        arena_id: impl Into<String>,
        kind: PlaybookKind,
        publisher: Arc<dyn Publisher<PlaybookEvent>>,
    ) {
        *self.attachment.lock().unwrap_or_else(|e| e.into_inner()) = Some(Attachment {
            arena_id: arena_id.into(),
            kind,
            publisher,
        });
        self.live_handles.store(0, Ordering::SeqCst);
        let announcement = {
            let mut progress = self.progress.lock().unwrap_or_else(|e| e.into_inner());
            *progress = Progress::fresh();
            progress.clone()
        };
        self.publish(announcement);
    }

    pub fn playbook_id(&self) -> &str {
        &self.playbook_id
    }

    pub fn playbook_type(&self) -> PlaybookType {
        self.playbook_type
    }

    pub fn runs(&self) -> usize {
        self.progress.lock().unwrap_or_else(|e| e.into_inner()).runs
    }

    pub fn state(&self) -> PlaybookState {
        self.progress.lock().unwrap_or_else(|e| e.into_inner()).state
    }

    pub fn faults(&self) -> Vec<Fault> {
        self.progress
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .faults
            .clone()
    }

    pub fn report(&self, next: PlaybookState) {
        let reported = {
            let mut progress = self.progress.lock().unwrap_or_else(|e| e.into_inner());
            match self.advance(&mut progress, next) {
                Some(reported) => reported,
                None => return,
            }
        };
        self.publish(reported);
    }

    pub fn enter(&self) {
        self.live_handles.fetch_add(1, Ordering::SeqCst);
    }

    pub fn exit(&self) {
        let held = self
            .live_handles
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |held| {
                Some(held.saturating_sub(1))
            })
            .unwrap_or(0);
        if held != 1 {
            return;
        }
        self.report_stopped();
    }

    pub fn release(&self) {
        self.live_handles.store(0, Ordering::SeqCst);
        self.report_stopped();
    }

    pub fn report_stopped(&self) {
        let reported = {
            let mut progress = self.progress.lock().unwrap_or_else(|e| e.into_inner());
            if progress.state != PlaybookState::Running {
                return;
            }
            match self.advance(&mut progress, PlaybookState::Stopped) {
                Some(reported) => reported,
                None => return,
            }
        };
        self.publish(reported);
    }

    pub fn report_running(&self) {
        let reported = {
            let mut progress = self.progress.lock().unwrap_or_else(|e| e.into_inner());
            progress.runs += 1;
            if progress.state == PlaybookState::Running {
                progress.clone()
            } else {
                progress.faults.clear();
                match self.advance(&mut progress, PlaybookState::Running) {
                    Some(reported) => reported,
                    None => return,
                }
            }
        };
        self.publish(reported);
    }

    pub fn report_fault(&self, fault: Fault) {
        let reported = {
            let mut progress = self.progress.lock().unwrap_or_else(|e| e.into_inner());
            progress.faults.push(fault);
            match self.advance(&mut progress, PlaybookState::Faulted) {
                Some(reported) => reported,
                None => return,
            }
        };
        self.publish(reported);
    }

    fn advance(&self, progress: &mut Progress, next: PlaybookState) -> Option<Progress> {
        if !PlaybookRule.is_legal(progress.state, next) {
            tracing::error!(
                playbook = %self.playbook_id,
                from = %progress.state,
                to = %next,
                phase = "illegal_playbook_transition",
                "refusing to report an illegal playbook transition"
            );
            return None;
        }
        progress.state = next;
        Some(progress.clone())
    }

    fn publish(&self, reported: Progress) {
        let attached = {
            let attachment = self.attachment.lock().unwrap_or_else(|e| e.into_inner());
            attachment.as_ref().map(|attachment| {
                (
                    attachment.arena_id.clone(),
                    attachment.kind,
                    Arc::clone(&attachment.publisher),
                )
            })
        };
        let Some((arena_id, kind, publisher)) = attached else {
            return;
        };
        publisher.publish(&PlaybookEvent::new(
            arena_id,
            self.playbook_id.clone(),
            self.playbook_type,
            kind,
            reported.state,
            reported.runs,
            reported.faults,
        ));
    }
}
