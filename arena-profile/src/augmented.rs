use crate::observer;
use crate::profiler::{
    CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch, ProfileEnvVar, ProfileSession,
    ShutdownSignal, FINISH_TIMEOUT,
};
use arena::lifecycle::RunnableState;
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

pub enum AugmentState {
    AsyncProfiler { scratch: crate::backend::ScratchDir, folded_path: PathBuf },
    PythonSite { scratch: crate::backend::ScratchDir },
}

pub trait ArgAugmentingSampler: Send + Sync {
    fn augment(&self, request: &LaunchRequest) -> Result<Augmentation, CpuProfileError>;

    fn collect(
        &self,
        state: AugmentState,
        budget: Duration,
        profiled_pid: u32,
        limit_bytes: usize,
    ) -> Result<String, CpuProfileError>;
}

pub fn augmenting_sampler_for(
    backend: CpuProfilerBackend,
    limit_bytes: usize,
) -> Box<dyn ArgAugmentingSampler> {
    match backend {
        CpuProfilerBackend::AsyncProfiler => Box::new(crate::backend::async_profiler::AsyncProfilerSampler),
        CpuProfilerBackend::PythonSiteSampler => {
            Box::new(crate::backend::python_site::PythonSiteSampler::new(limit_bytes))
        }
        CpuProfilerBackend::Perf | CpuProfilerBackend::PySpy | CpuProfilerBackend::DotnetTrace => {
            unreachable!("Perf/PySpy/DotnetTrace are wrapping backends, not arg-augmenting")
        }
    }
}

pub(crate) struct AugmentedProfileSession {
    sampler: Box<dyn ArgAugmentingSampler>,
    state: AugmentState,
    component_id: String,
    limit_bytes: usize,
}

impl ProfileSession for AugmentedProfileSession {
    fn sampling(&self) {
        observer::report(&self.component_id, RunnableState::Started);
    }

    fn finish(self: Box<Self>, augmented_process: &mut Child) -> Result<(), CpuProfileError> {
        let AugmentedProfileSession { sampler, state, component_id, limit_bytes } = *self;
        let profiled_pid = augmented_process.id();
        observer::report(&component_id, RunnableState::Stopping);
        let exited = crate::backend::wait_bounded(augmented_process, FINISH_TIMEOUT);
        let collected = sampler.collect(state, FINISH_TIMEOUT, profiled_pid, limit_bytes);
        let collected = match (exited, collected) {
            (Err(e), Err(collect_error)) => Err(CpuProfileError::Finish(format!(
                "profiled process did not exit cleanly ({e}) and no profile was collected: \
                 {collect_error}"
            ))),
            (_, collected) => collected,
        };
        match collected {
            Ok(folded) => {
                observer::report_complete(&component_id, &folded);
                Ok(())
            }
            Err(e) => {
                observer::report_faulted(&component_id, &e.to_string());
                Err(e)
            }
        }
    }
}

pub struct Augmentation {
    pub args: Vec<String>,
    pub env: Vec<ProfileEnvVar>,
    pub state: AugmentState,
}

pub fn prepare_augmented(
    sampler: Box<dyn ArgAugmentingSampler>,
    request: &LaunchRequest,
    component_id: String,
    limit_bytes: usize,
) -> Result<PreparedLaunch, CpuProfileError> {
    let Augmentation { args, env, state } = sampler.augment(request)?;
    Ok(PreparedLaunch::ArgsAugmented {
        args,
        env,
        shutdown_signal: ShutdownSignal::Terminate,
        session: Box::new(AugmentedProfileSession {
            sampler,
            state,
            component_id,
            limit_bytes,
        }),
    })
}
