use crate::observer;
use crate::profiler::{
    read_collected, CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch, ProfileSession,
    ShutdownSignal, FINISH_TIMEOUT,
};
use arena::lifecycle::RunnableState;
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

pub enum AugmentState {
    AsyncProfiler { folded_path: PathBuf },
}

pub trait ArgAugmentingSampler: Send + Sync {
    fn augment(
        &self,
        request: &LaunchRequest,
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
    component_id: String,
}

impl ProfileSession for AugmentedProfileSession {
    fn sampling(&self) {
        observer::report(&self.component_id, RunnableState::Started);
    }

    fn finish(self: Box<Self>, augmented_process: &mut Child) -> Result<(), CpuProfileError> {
        let AugmentedProfileSession { sampler, state, component_id } = *self;
        observer::report(&component_id, RunnableState::Stopping);
        let collected = crate::backend::wait_bounded(augmented_process, FINISH_TIMEOUT)
            .map_err(|e| CpuProfileError::Finish(format!("profiled process did not exit cleanly: {e}")))
            .and_then(|_status| sampler.collect(state, FINISH_TIMEOUT))
            .and_then(|folded_path| read_collected(&folded_path));
        match collected {
            Ok(folded) => {
                observer::report_complete(&component_id, &folded);
                Ok(())
            }
            Err(e) => {
                observer::report(&component_id, RunnableState::Faulted);
                Err(e)
            }
        }
    }
}

pub fn prepare_augmented(
    sampler: Box<dyn ArgAugmentingSampler>,
    request: &LaunchRequest,
    component_id: String,
) -> Result<PreparedLaunch, CpuProfileError> {
    let (args, state) = sampler.augment(request)?;
    Ok(PreparedLaunch::ArgsAugmented {
        args,
        shutdown_signal: ShutdownSignal::Terminate,
        session: Box::new(AugmentedProfileSession {
            sampler,
            state,
            component_id,
        }),
    })
}
