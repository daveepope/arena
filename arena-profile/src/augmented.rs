use crate::profiler::{
    render_collected, CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch, ProfileSession,
    ShutdownSignal, FINISH_TIMEOUT,
};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

pub enum AugmentState {
    AsyncProfiler { folded_path: PathBuf },
}

pub trait ArgAugmentingSampler: Send + Sync {
    fn augment(
        &self,
        request: &LaunchRequest,
        output_path: &Path,
    ) -> Result<(Vec<String>, AugmentState), CpuProfileError>;

    fn collect(&self, state: AugmentState, budget: Duration) -> Result<PathBuf, CpuProfileError>;
}

pub fn augmenting_sampler_for(backend: CpuProfilerBackend) -> Box<dyn ArgAugmentingSampler> {
    match backend {
        CpuProfilerBackend::AsyncProfiler => Box::new(crate::backend::async_profiler::AsyncProfilerSampler),
        CpuProfilerBackend::Perf | CpuProfilerBackend::PySpy | CpuProfilerBackend::DotnetTrace => {
            unreachable!("Perf/PySpy/DotnetTrace are wrapping backends, not arg-augmenting")
        }
    }
}

pub(crate) struct AugmentedProfileSession {
    sampler: Box<dyn ArgAugmentingSampler>,
    state: AugmentState,
    output_path: PathBuf,
    include_hotspots: bool,
}

impl ProfileSession for AugmentedProfileSession {
    fn with_hotspots(self: Box<Self>) -> Box<dyn ProfileSession> {
        Box::new(AugmentedProfileSession { include_hotspots: true, ..*self })
    }

    fn finish(self: Box<Self>, augmented_process: &mut Child) -> Result<(), CpuProfileError> {
        crate::backend::wait_bounded(augmented_process, FINISH_TIMEOUT)
            .map_err(|e| CpuProfileError::Finish(format!("profiled process did not exit cleanly: {e}")))?;
        let folded_path = self.sampler.collect(self.state, FINISH_TIMEOUT)?;
        render_collected(&folded_path, &self.output_path, self.include_hotspots)
    }
}

pub fn prepare_augmented(
    sampler: Box<dyn ArgAugmentingSampler>,
    request: &LaunchRequest,
    output_path: PathBuf,
) -> Result<PreparedLaunch, CpuProfileError> {
    let (args, state) = sampler.augment(request, &output_path)?;
    Ok(PreparedLaunch::ArgsAugmented {
        args,
        shutdown_signal: ShutdownSignal::Terminate,
        session: Box::new(AugmentedProfileSession {
            sampler,
            state,
            output_path,
            include_hotspots: false,
        }),
    })
}
