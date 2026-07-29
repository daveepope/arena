use arena_profile::{prepare_cpu_profile, CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch, RenderError};
use std::path::PathBuf;

fn temp_html_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("arena-profile-profiler-test-{name}-{}.html", std::process::id()))
}

fn sample_request() -> LaunchRequest {
    LaunchRequest {
        program: PathBuf::from("/bin/target"),
        args: vec!["a".to_string(), "b".to_string()],
    }
}

#[test]
fn cpu_profile_error_display_spawn_includes_underlying_io_error() {
    let e = CpuProfileError::Spawn(std::io::Error::other("spawn boom"));

    assert!(e.to_string().contains("spawn boom"));
}

#[test]
fn cpu_profile_error_display_launch_includes_message() {
    let e = CpuProfileError::Launch("bad launch config".to_string());

    assert!(e.to_string().contains("bad launch config"));
}

#[test]
fn cpu_profile_error_display_finish_includes_message() {
    let e = CpuProfileError::Finish("shutdown timed out".to_string());

    assert!(e.to_string().contains("shutdown timed out"));
}

#[test]
fn cpu_profile_error_display_missing_binary_includes_binary_and_hint() {
    let e = CpuProfileError::MissingBinary {
        binary: "perf",
        install_hint: "install linux-tools",
    };

    let message = e.to_string();
    assert!(message.contains("perf"));
    assert!(message.contains("install linux-tools"));
}

#[test]
fn cpu_profile_error_display_render_includes_underlying_render_error() {
    let e = CpuProfileError::Render(RenderError::Inferno("bad stacks".to_string()));

    assert!(e.to_string().contains("bad stacks"));
}

#[test]
fn cpu_profile_error_from_render_error_wraps_as_render_variant() {
    let e: CpuProfileError = RenderError::Io(std::io::Error::other("boom")).into();

    assert!(matches!(e, CpuProfileError::Render(_)));
}

#[test]
fn prepare_cpu_profile_perf_backend_returns_wrapped_variant() {
    let result = prepare_cpu_profile(
        CpuProfilerBackend::Perf,
        sample_request(),
        temp_html_path("dispatch-perf"),
    );

    match result {
        Ok(PreparedLaunch::Wrapped { .. }) => {}
        Err(CpuProfileError::MissingBinary { .. }) => {}
        Ok(PreparedLaunch::ArgsAugmented { .. }) => panic!("expected Wrapped variant"),
        Err(other) => panic!("expected MissingBinary, got {other}"),
    }
}

#[test]
fn prepare_cpu_profile_async_profiler_backend_returns_args_augmented_variant() {
    let result = prepare_cpu_profile(
        CpuProfilerBackend::AsyncProfiler,
        sample_request(),
        temp_html_path("dispatch-async-profiler"),
    );

    match result {
        Ok(PreparedLaunch::ArgsAugmented { .. }) => {}
        Err(CpuProfileError::MissingBinary { .. }) => {}
        Ok(PreparedLaunch::Wrapped { .. }) => panic!("expected ArgsAugmented variant"),
        Err(other) => panic!("expected MissingBinary, got {other}"),
    }
}
