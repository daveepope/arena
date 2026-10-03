use crate::profiler::{
    render_collected, CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch, ProfileSession,
    FINISH_TIMEOUT,
};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

pub enum WrapState {
    Perf { data_path: PathBuf },
    PySpy { folded_path: PathBuf },
    DotnetTrace { nettrace_path: PathBuf },
}

pub trait WrappingSampler: Send + Sync {
    fn wrap(
        &self,
        request: &LaunchRequest,
        output_path: &Path,
    ) -> Result<(PathBuf, Vec<String>, WrapState), CpuProfileError>;

    fn collect(
        &self,
        state: WrapState,
        wrapping_child: &mut Child,
        budget: Duration,
    ) -> Result<PathBuf, CpuProfileError>;
}

pub fn wrapping_sampler_for(backend: CpuProfilerBackend) -> Box<dyn WrappingSampler> {
    match backend {
        CpuProfilerBackend::Perf => Box::new(crate::backend::perf::PerfSampler),
        CpuProfilerBackend::PySpy => Box::new(crate::backend::pyspy::PySpySampler),
        CpuProfilerBackend::DotnetTrace => Box::new(crate::backend::dotnet_trace::DotnetTraceSampler),
        CpuProfilerBackend::AsyncProfiler => {
            unreachable!("AsyncProfiler is an arg-augmenting backend, not wrapping")
        }
    }
}

pub(crate) struct WrappedProfileSession {
    sampler: Box<dyn WrappingSampler>,
    state: WrapState,
    output_path: PathBuf,
    include_hotspots: bool,
}

impl ProfileSession for WrappedProfileSession {
    fn with_hotspots(self: Box<Self>) -> Box<dyn ProfileSession> {
        Box::new(WrappedProfileSession { include_hotspots: true, ..*self })
    }

    fn finish(self: Box<Self>, wrapping_child: &mut Child) -> Result<(), CpuProfileError> {
        let folded_path = self
            .sampler
            .collect(self.state, wrapping_child, FINISH_TIMEOUT)?;
        render_collected(&folded_path, &self.output_path, self.include_hotspots)
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
        session: Box::new(WrappedProfileSession {
            sampler,
            state,
            output_path,
            include_hotspots: false,
        }),
    })
}
