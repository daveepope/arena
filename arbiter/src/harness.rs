use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::complexity::ComplexityDelta;
use crate::cursor::CursorHarness;
use crate::features::Features;
use crate::scoring::{DebtSample, Report};

pub trait Harness {
    fn start(&mut self) -> notify::Result<()>;
    fn stop(&mut self);
    fn features(&self) -> Features;
    fn calculate(&self) -> Report;
    fn complexity_deltas(&self) -> Vec<ComplexityDelta>;
    fn debt_series(&self) -> Vec<DebtSample>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentHarness {
    Cursor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsupportedHarness(pub String);

impl fmt::Display for UnsupportedHarness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "unsupported agent harness '{}' (supported: cursor)",
            self.0
        )
    }
}

impl Error for UnsupportedHarness {}

impl AgentHarness {
    pub fn parse(value: &str) -> Result<Self, UnsupportedHarness> {
        match value.trim().to_lowercase().as_str() {
            "cursor" => Ok(AgentHarness::Cursor),
            other => Err(UnsupportedHarness(other.to_string())),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            AgentHarness::Cursor => "cursor",
        }
    }

    pub fn open(self, path: PathBuf) -> Box<dyn Harness> {
        let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self.open_in(path, workspace_root)
    }

    pub fn open_in(self, path: PathBuf, workspace_root: PathBuf) -> Box<dyn Harness> {
        match self {
            AgentHarness::Cursor => Box::new(CursorHarness::new(path, workspace_root)),
        }
    }

    pub fn wait_for_new_session(self) -> io::Result<PathBuf> {
        match self {
            AgentHarness::Cursor => crate::cursor::wait_for_new_session(),
        }
    }

    pub fn latest_session(self) -> io::Result<PathBuf> {
        match self {
            AgentHarness::Cursor => crate::cursor::latest_session(),
        }
    }
}
