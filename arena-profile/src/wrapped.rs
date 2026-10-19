use crate::observer;
use crate::profiler::{
    CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch, ProfileSession,
    FINISH_TIMEOUT,
};
use arena::lifecycle::RunnableState;
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

pub enum WrapState {
    Perf { scratch: crate::backend::ScratchDir, data_path: PathBuf },
    PySpy { scratch: crate::backend::ScratchDir, folded_path: PathBuf },
    DotnetTrace { scratch: crate::backend::ScratchDir, nettrace_path: PathBuf },
}

pub trait WrappingSampler: Send + Sync {
    fn wrap(
        &self,
        request: &LaunchRequest,
    ) -> Result<(PathBuf, Vec<String>, WrapState), CpuProfileError>;

    fn collect(
        &self,
        state: WrapState,
        wrapping_child: &mut Child,
        budget: Duration,
        limit_bytes: usize,
    ) -> Result<Option<String>, CpuProfileError>;
}

pub fn wrapping_sampler_for(backend: CpuProfilerBackend) -> Box<dyn WrappingSampler> {
    match backend {
        CpuProfilerBackend::Perf => Box::new(crate::backend::perf::PerfSampler),
        CpuProfilerBackend::PySpy => Box::new(crate::backend::pyspy::PySpySampler),
        CpuProfilerBackend::DotnetTrace => Box::new(crate::backend::dotnet_trace::DotnetTraceSampler),
        CpuProfilerBackend::AsyncProfiler | CpuProfilerBackend::PythonSiteSampler => {
            unreachable!("AsyncProfiler and PythonSiteSampler are arg-augmenting backends, not wrapping")
        }
    }
}

pub(crate) struct WrappedProfileSession {
    sampler: Box<dyn WrappingSampler>,
    state: WrapState,
    component_id: String,
    limit_bytes: usize,
}

impl ProfileSession for WrappedProfileSession {
    fn sampling(&self) {
        observer::report(&self.component_id, RunnableState::Started);
    }

    fn finish(self: Box<Self>, wrapping_child: &mut Child) -> Result<(), CpuProfileError> {
        let WrappedProfileSession { sampler, state, component_id, limit_bytes } = *self;
        observer::report(&component_id, RunnableState::Stopping);
        let collected = sampler.collect(state, wrapping_child, FINISH_TIMEOUT, limit_bytes);
        match collected {
            Ok(Some(folded)) => {
                observer::report_complete(&component_id, &folded);
                Ok(())
            }
            Ok(None) => {
                observer::report(&component_id, RunnableState::Stopped);
                Ok(())
            }
            Err(e) => {
                observer::report(&component_id, RunnableState::Faulted);
                Err(e)
            }
        }
    }
}

pub fn prepare_wrapped(
    sampler: Box<dyn WrappingSampler>,
    request: &LaunchRequest,
    component_id: String,
    limit_bytes: usize,
) -> Result<PreparedLaunch, CpuProfileError> {
    let (program, args, state) = sampler.wrap(request)?;
    Ok(PreparedLaunch::Wrapped {
        program,
        args,
        session: Box::new(WrappedProfileSession {
            sampler,
            state,
            component_id,
            limit_bytes,
        }),
    })
}
