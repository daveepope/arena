use crate::render::{render_folded_to_html, RenderError};
use crate::sampler::{self, ArgAugmentingSampler, AugmentState, WrapState, WrappingSampler};
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

pub const FINISH_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuProfilerBackend {
    Perf,
    AsyncProfiler,
    PySpy,
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

pub struct WrappedProfileSession {
    sampler: Box<dyn WrappingSampler>,
    state: WrapState,
    output_path: PathBuf,
    include_hotspots: bool,
}

impl WrappedProfileSession {
    pub fn with_hotspots(mut self) -> Self {
        self.include_hotspots = true;
        self
    }

    pub fn finish(self, wrapping_child: &mut Child) -> Result<(), CpuProfileError> {
        let folded_path = self
            .sampler
            .collect(self.state, wrapping_child, FINISH_TIMEOUT)?;
        render_collected(&folded_path, &self.output_path, self.include_hotspots)
    }
}

pub struct AugmentedProfileSession {
    sampler: Box<dyn ArgAugmentingSampler>,
    state: AugmentState,
    output_path: PathBuf,
    include_hotspots: bool,
}

impl AugmentedProfileSession {
    pub fn with_hotspots(mut self) -> Self {
        self.include_hotspots = true;
        self
    }

    pub fn finish(self, augmented_process: &mut Child) -> Result<(), CpuProfileError> {
        crate::backend::wait_bounded(augmented_process, FINISH_TIMEOUT).map_err(|e| {
            CpuProfileError::Finish(format!("profiled process did not exit cleanly: {e}"))
        })?;
        let folded_path = self.sampler.collect(self.state, FINISH_TIMEOUT)?;
        render_collected(&folded_path, &self.output_path, self.include_hotspots)
    }
}

pub enum PreparedLaunch {
    Wrapped {
        program: PathBuf,
        args: Vec<String>,
        session: WrappedProfileSession,
    },
    Augmented {
        args: Vec<String>,
        env_vars: Vec<(String, String)>,
        shutdown_signal: ShutdownSignal,
        session: AugmentedProfileSession,
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
            PreparedLaunch::Augmented { args, env_vars, shutdown_signal, session } => {
                PreparedLaunch::Augmented {
                    args,
                    env_vars,
                    shutdown_signal,
                    session: session.with_hotspots(),
                }
            }
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
        CpuProfilerBackend::Perf | CpuProfilerBackend::PySpy => {
            prepare_wrapped(sampler::wrapping_sampler_for(backend), &request, output_path)
        }
        CpuProfilerBackend::AsyncProfiler => prepare_augmented(
            sampler::augmenting_sampler_for(backend),
            &request,
            output_path,
        ),
    }
}

pub fn prepare_wrapped(
    sampler: Box<dyn WrappingSampler>,
    request: &LaunchRequest,
    output_path: PathBuf,
) -> Result<PreparedLaunch, CpuProfileError> {
    let (program, args, state) = sampler.wrap(request, &output_path)?;
    Ok(PreparedLaunch::Wrapped {
        program,
        args,
        session: WrappedProfileSession {
            sampler,
            state,
            output_path,
            include_hotspots: false,
        },
    })
}

pub fn prepare_augmented(
    sampler: Box<dyn ArgAugmentingSampler>,
    request: &LaunchRequest,
    output_path: PathBuf,
) -> Result<PreparedLaunch, CpuProfileError> {
    let (args, env_vars, state) = sampler.augment(request, &output_path)?;
    Ok(PreparedLaunch::Augmented {
        args,
        env_vars,
        shutdown_signal: ShutdownSignal::Terminate,
        session: AugmentedProfileSession {
            sampler,
            state,
            output_path,
            include_hotspots: false,
        },
    })
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
    let render_result = render_folded_to_html(folded_file, output_path, include_hotspots);
    let _ = std::fs::remove_file(folded_path);
    render_result?;
    Ok(())
}
