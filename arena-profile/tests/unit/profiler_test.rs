#[path = "../support/recorded_profiles.rs"]
mod recorded_profiles;

use arena::lifecycle::RunnableState;
use arena_profile::profiler::read_collected;
use arena_profile::DEFAULT_CPU_PROFILE_LIMIT_BYTES;
use arena_profile::{prepare_cpu_profile, CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch};
use recorded_profiles::start_recording;
use std::path::PathBuf;

fn sample_request() -> LaunchRequest {
    LaunchRequest {
        program: PathBuf::from("/bin/target"),
        args: vec!["a".to_string(), "b".to_string()],
    }
}

fn write_sample_folded_stacks(name: &str) -> PathBuf {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let unique_id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "arena-profile-profiler-test-folded-{name}-{}-{unique_id}.folded",
        std::process::id()
    ));
    std::fs::write(&path, "main;handler;compute 42\n").expect("write sample folded stacks");
    path
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
fn prepare_cpu_profile_perf_backend_returns_wrapped_variant() {
    let result = prepare_cpu_profile(
        CpuProfilerBackend::Perf,
        sample_request(),
        "dispatch-perf",
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
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
        "dispatch-async-profiler",
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    );

    match result {
        Ok(PreparedLaunch::ArgsAugmented { .. }) => {}
        Err(CpuProfileError::MissingBinary { .. }) => {}
        Ok(PreparedLaunch::Wrapped { .. }) => panic!("expected ArgsAugmented variant"),
        Err(other) => panic!("expected MissingBinary, got {other}"),
    }
}

#[test]
fn read_collected_missing_folded_path_returns_finish_error() {
    let missing = std::env::temp_dir().join(format!(
        "arena-profile-profiler-test-missing-folded-{}.folded",
        std::process::id()
    ));

    let result = read_collected(&missing, DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
}

#[test]
fn read_collected_valid_folded_path_returns_stacks_and_removes_the_file() {
    let folded_path = write_sample_folded_stacks("read-collected-cleanup");

    let result = read_collected(&folded_path, DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    assert_eq!(result.unwrap(), "main;handler;compute 42\n");
    assert!(!folded_path.exists());
}

#[test]
fn prepare_cpu_profile_any_backend_reports_the_attaching_transition() {
    let recording = start_recording();

    let _ = prepare_cpu_profile(CpuProfilerBackend::Perf, sample_request(), "attaching", DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    assert_eq!(
        recording.recorded.states_for("attaching").first(),
        Some(&RunnableState::Starting)
    );
}

#[test]
fn prepare_cpu_profile_missing_backend_binary_reports_faulted_after_attaching() {
    let recording = start_recording();

    let result = prepare_cpu_profile(CpuProfilerBackend::Perf, sample_request(), "attach-fault", DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    if matches!(result, Err(CpuProfileError::MissingBinary { .. })) {
        assert_eq!(
            recording.recorded.states_for("attach-fault"),
            vec![RunnableState::Starting, RunnableState::Faulted]
        );
    } else {
        assert_eq!(
            recording.recorded.states_for("attach-fault"),
            vec![RunnableState::Starting]
        );
    }
}

#[test]
fn read_collected_stacks_over_the_limit_returns_profile_too_large() {
    let folded_path = write_sample_folded_stacks("read-collected-over-limit");
    let expected_bytes = std::fs::metadata(&folded_path).expect("metadata").len();

    let result = read_collected(&folded_path, 4);

    match result {
        Err(CpuProfileError::ProfileTooLarge { bytes, limit }) => {
            assert_eq!(bytes, expected_bytes);
            assert_eq!(limit, 4);
        }
        other => panic!("expected ProfileTooLarge, got {other:?}"),
    }
    assert!(!folded_path.exists());
}

#[test]
fn read_collected_stacks_exactly_at_the_limit_returns_the_stacks() {
    let folded_path = write_sample_folded_stacks("read-collected-at-limit");
    let limit = std::fs::metadata(&folded_path).expect("metadata").len() as usize;

    let result = read_collected(&folded_path, limit);

    assert_eq!(result.unwrap(), "main;handler;compute 42\n");
}

#[test]
fn profile_too_large_display_names_the_limit_and_how_to_raise_it() {
    let message = CpuProfileError::ProfileTooLarge {
        bytes: 99,
        limit: 8,
    }
    .to_string();

    assert!(message.contains("99 bytes"), "got: {message}");
    assert!(message.contains("8 byte limit"), "got: {message}");
    assert!(message.contains("lifecycle dashboard builder"), "got: {message}");
}
