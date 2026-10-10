use crate::render::RenderError;
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

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
    Render(RenderError),
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
            CpuProfileError::Render(e) => write!(f, "failed to render flamegraph report: {e}"),
        }
    }
}

impl std::error::Error for CpuProfileError {}

impl From<RenderError> for CpuProfileError {
    fn from(e: RenderError) -> Self {
        CpuProfileError::Render(e)
    }
}

pub struct LaunchRequest {
    pub program: PathBuf,
    pub args: Vec<String>,
}

pub trait ProfileSession: Send + Sync {
    fn with_hotspots(self: Box<Self>) -> Box<dyn ProfileSession>;
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

impl PreparedLaunch {
    pub fn with_hotspots(self) -> Self {
        match self {
            PreparedLaunch::Wrapped { program, args, session } => PreparedLaunch::Wrapped {
                program,
                args,
                session: session.with_hotspots(),
            },
            PreparedLaunch::ArgsAugmented { args, shutdown_signal, session } => PreparedLaunch::ArgsAugmented {
                args,
                shutdown_signal,
                session: session.with_hotspots(),
            },
        }
    }
}

pub fn prepare_cpu_profile(
    backend: CpuProfilerBackend,
    request: LaunchRequest,
    output_path: impl Into<PathBuf>,
) -> Result<PreparedLaunch, CpuProfileError> {
    let output_path = output_path.into();
    match backend {
        CpuProfilerBackend::Perf | CpuProfilerBackend::PySpy | CpuProfilerBackend::DotnetTrace => crate::wrapped::prepare_wrapped(
            crate::wrapped::wrapping_sampler_for(backend),
            &request,
            output_path,
        ),
        CpuProfilerBackend::AsyncProfiler => crate::augmented::prepare_augmented(
            crate::augmented::augmenting_sampler_for(backend),
            &request,
            output_path,
        ),
    }
}

pub fn render_collected(
    folded_path: &std::path::Path,
    output_path: &std::path::Path,
    include_hotspots: bool,
) -> Result<(), CpuProfileError> {
    let folded_file = std::fs::File::open(folded_path).map_err(|e| {
        CpuProfileError::Finish(format!(
            "failed to read folded stacks at {}: {e}",
            folded_path.display()
        ))
    })?;
    let render_result = crate::render::render_folded_to_html(folded_file, output_path, include_hotspots);
    let _ = std::fs::remove_file(folded_path);
    render_result?;
    Ok(())
}
