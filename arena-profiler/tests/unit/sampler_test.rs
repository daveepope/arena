use arena_profiler::sampler::{augmenting_sampler_for, wrapping_sampler_for};
use arena_profiler::CpuProfilerBackend;

#[test]
fn wrapping_sampler_for_perf_and_pyspy_returns_sampler() {
    let _ = wrapping_sampler_for(CpuProfilerBackend::Perf);
    let _ = wrapping_sampler_for(CpuProfilerBackend::PySpy);
}

#[test]
#[should_panic(expected = "AsyncProfiler is an arg-augmenting backend")]
fn wrapping_sampler_for_async_profiler_panics_unreachable() {
    let _ = wrapping_sampler_for(CpuProfilerBackend::AsyncProfiler);
}

#[test]
fn augmenting_sampler_for_async_profiler_returns_sampler() {
    let _ = augmenting_sampler_for(CpuProfilerBackend::AsyncProfiler);
}

#[test]
#[should_panic(expected = "Perf/PySpy are wrapping backends")]
fn augmenting_sampler_for_perf_panics_unreachable() {
    let _ = augmenting_sampler_for(CpuProfilerBackend::Perf);
}

#[test]
#[should_panic(expected = "Perf/PySpy are wrapping backends")]
fn augmenting_sampler_for_pyspy_panics_unreachable() {
    let _ = augmenting_sampler_for(CpuProfilerBackend::PySpy);
}
