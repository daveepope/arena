use crate::observer;
use crate::profiler::{
    read_collected, CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch, ProfileSession,
    FINISH_TIMEOUT,
};
use arena::lifecycle::RunnableState;
use std::path::PathBuf;
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
    ) -> Result<(PathBuf, Vec<String>, WrapState), CpuProfileError>;

    fn collect(
        &self,
        state: WrapState,
        wrapping_child: &mut Child,
        budget: Duration,
    ) -> Result<Option<PathBuf>, CpuProfileError>;
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
        let collected = sampler
            .collect(state, wrapping_child, FINISH_TIMEOUT)
            .and_then(|folded_path| match folded_path {
                Some(folded_path) => read_collected(&folded_path, limit_bytes).map(Some),
                None => Ok(None),
            });
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
