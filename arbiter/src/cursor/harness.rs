use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::complexity::{ComplexityDelta, ComplexityEngine};
use crate::features::Features;
use crate::harness::Harness;
use crate::scoring::{report, DebtSample, Report};
use crate::session::SessionEngine;

use super::transcript::parse_line;

pub struct CursorHarness {
    engine: SessionEngine,
    complexity: ComplexityEngine,
    history: Arc<Mutex<Vec<DebtSample>>>,
}

impl CursorHarness {
    pub fn new(path: PathBuf, workspace_root: PathBuf) -> Self {
        Self {
            engine: SessionEngine::new(path, parse_line),
            complexity: ComplexityEngine::new(workspace_root),
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

impl Harness for CursorHarness {
    fn start(&mut self) -> notify::Result<()> {
        self.engine.start()?;
        self.complexity.start()
    }

    fn stop(&mut self) {
        self.engine.stop();
        self.complexity.stop();
    }

    fn features(&self) -> Features {
        let mut features = self.engine.features();
        features.bytes_delta = self.complexity.bytes_delta();
        features.files_delta = self.complexity.files_delta();
        features.complexity_introduced = self.complexity.introduced();
        features
    }

    fn calculate(&self) -> Report {
        let report = report(self.features());
        if let Ok(mut guard) = self.history.lock() {
            guard.push(DebtSample {
                at_ms: now_ms(),
                debt: report.debt,
            });
        }
        report
    }

    fn complexity_deltas(&self) -> Vec<ComplexityDelta> {
        self.complexity.deltas()
    }

    fn debt_series(&self) -> Vec<DebtSample> {
        self.history
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}
