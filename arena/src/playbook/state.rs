use std::fmt;

use crate::lifecycle::rule::LifecycleRule;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybookState {
    Registered,
    Running,
    Stopped,
    Faulted,
}

impl PlaybookState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlaybookState::Registered => "registered",
            PlaybookState::Running => "running",
            PlaybookState::Stopped => "stopped",
            PlaybookState::Faulted => "faulted",
        }
    }
}

impl fmt::Display for PlaybookState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub struct PlaybookRule;

impl LifecycleRule<PlaybookState> for PlaybookRule {
    fn is_legal(&self, from: PlaybookState, to: PlaybookState) -> bool {
        use PlaybookState::*;
        match (from, to) {
            (_, Faulted) => true,
            (Registered | Stopped | Faulted, Running) => true,
            (Running, Stopped) => true,
            _ => false,
        }
    }
}
