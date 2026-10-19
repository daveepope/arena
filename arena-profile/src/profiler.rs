use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

use arena::lifecycle::RunnableState;

use crate::observer;

pub const FINISH_TIMEOUT: Duration = Duration::from_secs(10);

pub const DEFAULT_CPU_PROFILE_LIMIT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuProfilerBackend {
    Perf,
    AsyncProfiler,
    PySpy,
    PythonSiteSampler,
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
    ProfileTooLarge { bytes: u64, limit: u64 },
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
            CpuProfileError::ProfileTooLarge { bytes, limit } => write!(
                f,
                "captured cpu profile is {bytes} bytes, over the {limit} byte limit; raise the cpu profile limit on the lifecycle dashboard builder"
            ),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvMerge {
    Space,
    PathList,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileEnvVar {
    pub key: String,
    pub value: String,
    pub merge: EnvMerge,
}

impl ProfileEnvVar {
    pub fn space_joined(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self { key: key.into(), value: value.into(), merge: EnvMerge::Space }
    }

    pub fn path_joined(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self { key: key.into(), value: value.into(), merge: EnvMerge::PathList }
    }
}

pub enum PreparedLaunch {
    Wrapped {
        program: PathBuf,
        args: Vec<String>,
        session: Box<dyn ProfileSession>,
    },
    ArgsAugmented {
        args: Vec<String>,
        env: Vec<ProfileEnvVar>,
        shutdown_signal: ShutdownSignal,
        session: Box<dyn ProfileSession>,
    },
}

pub fn prepare_cpu_profile(
    backend: CpuProfilerBackend,
    request: LaunchRequest,
    component_id: impl Into<String>,
    limit_bytes: usize,
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
                limit_bytes,
            )
        }
        CpuProfilerBackend::AsyncProfiler | CpuProfilerBackend::PythonSiteSampler => {
            crate::augmented::prepare_augmented(
            crate::augmented::augmenting_sampler_for(backend, limit_bytes),
            &request,
            component_id.clone(),
            limit_bytes,
        )
        }
    };

    if prepared.is_err() {
        observer::report(&component_id, RunnableState::Faulted);
    }
    prepared
}

pub fn read_collected(
    folded_path: &std::path::Path,
    limit_bytes: usize,
) -> Result<String, CpuProfileError> {
    tracing::debug!(folded = %folded_path.display(), "reading collected profile samples");
    let folded = read_within_limit(folded_path, limit_bytes);
    let _ = std::fs::remove_file(folded_path);
    folded
}

fn read_within_limit(
    folded_path: &std::path::Path,
    limit_bytes: usize,
) -> Result<String, CpuProfileError> {
    let unreadable = |e: std::io::Error| {
        CpuProfileError::Finish(format!(
            "failed to read folded stacks at {}: {e}",
            folded_path.display()
        ))
    };
    let bytes = std::fs::metadata(folded_path).map_err(unreadable)?.len();
    if bytes > limit_bytes as u64 {
        return Err(CpuProfileError::ProfileTooLarge {
            bytes,
            limit: limit_bytes as u64,
        });
    }
    std::fs::read_to_string(folded_path).map_err(unreadable)
}
