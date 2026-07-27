use crate::profiler::{CpuProfileError, CpuProfilerBackend, LaunchRequest};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

pub(crate) enum WrapState {
    Perf { data_path: PathBuf },
    PySpy { folded_path: PathBuf },
}

pub(crate) enum AugmentState {
    AsyncProfiler { folded_path: PathBuf },
}

pub(crate) trait WrappingSampler: Send + Sync {
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

pub(crate) trait ArgAugmentingSampler: Send + Sync {
    fn augment(
        &self,
        request: &LaunchRequest,
        output_path: &Path,
    ) -> Result<(Vec<String>, AugmentState), CpuProfileError>;

    fn collect(&self, state: AugmentState, budget: Duration) -> Result<PathBuf, CpuProfileError>;
}

pub(crate) fn wrapping_sampler_for(backend: CpuProfilerBackend) -> Box<dyn WrappingSampler> {
    match backend {
        CpuProfilerBackend::Perf => Box::new(crate::backend::perf::PerfSampler),
        CpuProfilerBackend::PySpy => Box::new(crate::backend::pyspy::PySpySampler),
        CpuProfilerBackend::AsyncProfiler => {
            unreachable!("AsyncProfiler is an arg-augmenting backend, not wrapping")
        }
    }
}

pub(crate) fn augmenting_sampler_for(backend: CpuProfilerBackend) -> Box<dyn ArgAugmentingSampler> {
    match backend {
        CpuProfilerBackend::AsyncProfiler => Box::new(crate::backend::async_profiler::AsyncProfilerSampler),
        CpuProfilerBackend::Perf | CpuProfilerBackend::PySpy => {
            unreachable!("Perf/PySpy are wrapping backends, not arg-augmenting")
        }
    }
}
