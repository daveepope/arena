use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

use arena::lifecycle::RunnableState;

use crate::observer;

pub const FINISH_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuProfilerBackend {
    Perf,
    AsyncProfiler,
    PySpy,
    DotnetTrace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownSignal {
    Terminate,
    Kill,
}

#[derive(Debug)]
pub enum CpuProfileError {
    MissingBinary {
        binary: &'static str,
        install_hint: &'static str,
    },
    Spawn(std::io::Error),
    Launch(String),
    Finish(String),
}

impl std::fmt::Display for CpuProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CpuProfileError::MissingBinary { binary, install_hint } => {
                write!(f, "required profiling tool '{binary}' was not found on PATH ({install_hint})")
            }
            CpuProfileError::Spawn(e) => write!(f, "failed to spawn profiler process: {e}"),
            CpuProfileError::Launch(msg) => write!(f, "failed to prepare profiled launch: {msg}"),
            CpuProfileError::Finish(msg) => write!(f, "failed to finish profiling session: {msg}"),
        }
    }
}

impl std::error::Error for CpuProfileError {}

pub struct LaunchRequest {
    pub program: PathBuf,
    pub args: Vec<String>,
}

pub trait ProfileSession: Send + Sync {
    fn sampling(&self);
    fn finish(self: Box<Self>, child: &mut Child) -> Result<(), CpuProfileError>;
}

pub enum PreparedLaunch {
    Wrapped {
        program: PathBuf,
        args: Vec<String>,
        session: Box<dyn ProfileSession>,
    },
    ArgsAugmented {
        args: Vec<String>,
        shutdown_signal: ShutdownSignal,
        session: Box<dyn ProfileSession>,
    },
}

pub fn prepare_cpu_profile(
    backend: CpuProfilerBackend,
    request: LaunchRequest,
    component_id: impl Into<String>,
) -> Result<PreparedLaunch, CpuProfileError> {
    let component_id = component_id.into();
    tracing::info!(backend = ?backend, component = %component_id, "attaching cpu profiler");
    observer::report(&component_id, RunnableState::Starting);

    let prepared = match backend {
        CpuProfilerBackend::Perf | CpuProfilerBackend::PySpy | CpuProfilerBackend::DotnetTrace => {
            crate::wrapped::prepare_wrapped(
                crate::wrapped::wrapping_sampler_for(backend),
                &request,
                component_id.clone(),
            )
        }
        CpuProfilerBackend::AsyncProfiler => crate::augmented::prepare_augmented(
            crate::augmented::augmenting_sampler_for(backend),
            &request,
            component_id.clone(),
        ),
    };

    if prepared.is_err() {
        observer::report(&component_id, RunnableState::Faulted);
    }
    prepared
}

pub fn read_collected(folded_path: &std::path::Path) -> Result<String, CpuProfileError> {
    tracing::debug!(folded = %folded_path.display(), "reading collected profile samples");
    let folded = std::fs::read_to_string(folded_path).map_err(|e| {
        CpuProfileError::Finish(format!(
            "failed to read folded stacks at {}: {e}",
            folded_path.display()
        ))
    });
    let _ = std::fs::remove_file(folded_path);
    folded
}
