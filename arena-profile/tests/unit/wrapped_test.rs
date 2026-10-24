#[path = "../support/recorded_profiles.rs"]
mod recorded_profiles;

use arena::lifecycle::RunnableState;
use arena_profile::wrapped::{prepare_wrapped, wrapping_sampler_for, WrapState, WrappingSampler};
use arena_profile::DEFAULT_CPU_PROFILE_LIMIT_BYTES;
use arena_profile::{CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch};
use recorded_profiles::start_recording;
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

enum WrapOutcome {
    WrapFails(CpuProfileError),
    CollectFails,
    CollectSucceeds,
    CollectFindsNoSamples,
}

struct FakeWrappingSampler {
    outcome: WrapOutcome,
    wrapped_program: &'static str,
}

impl WrappingSampler for FakeWrappingSampler {
    fn wrap(
        &self,
        request: &LaunchRequest,
    ) -> Result<(PathBuf, Vec<String>, WrapState), CpuProfileError> {
        match &self.outcome {
            WrapOutcome::WrapFails(e) => Err(clone_error(e)),
            WrapOutcome::CollectFails
            | WrapOutcome::CollectSucceeds
            | WrapOutcome::CollectFindsNoSamples => {
                let mut args = vec!["--".to_string(), request.program.to_string_lossy().into_owned()];
                args.extend(request.args.iter().cloned());
                Ok((
                    PathBuf::from(self.wrapped_program),
                    args,
                    WrapState::Perf {
                        scratch: arena_profile::backend::ScratchDir::create("arena-profile-wrapped-test")
                            .expect("scratch"),
                        data_path: PathBuf::from("/tmp/unused.data"),
                    },
                ))
            }
        }
    }

    fn collect(
        &self,
        _state: WrapState,
        _wrapping_child: &mut Child,
        _budget: Duration,
        limit_bytes: usize,
    ) -> Result<String, CpuProfileError> {
        match &self.outcome {
            WrapOutcome::CollectFails => Err(CpuProfileError::Finish("collect failed".into())),
            WrapOutcome::CollectSucceeds => {
                let path = write_sample_folded_stacks("wrapped-collect-succeeds");
                arena_profile::profiler::read_collected(&path, limit_bytes)
            }
            WrapOutcome::CollectFindsNoSamples => {
                Err(CpuProfileError::Finish("no samples were recorded".into()))
            }
            WrapOutcome::WrapFails(_) => unreachable!("collect() called after wrap() failed"),
        }
    }
}

fn write_sample_folded_stacks(name: &str) -> PathBuf {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let unique_id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "arena-profile-wrapped-test-folded-{name}-{}-{unique_id}.folded",
        std::process::id()
    ));
    std::fs::write(&path, "main;handler;compute 42\n").expect("write sample folded stacks");
    path
}

fn clone_error(e: &CpuProfileError) -> CpuProfileError {
    match e {
        CpuProfileError::MissingBinary { binary, install_hint } => {
            CpuProfileError::MissingBinary { binary, install_hint }
        }
        _ => unreachable!("clone_error only used for MissingBinary in these tests"),
    }
}

fn sample_request() -> LaunchRequest {
    LaunchRequest {
        program: PathBuf::from("/bin/target"),
        args: vec!["a".to_string(), "b".to_string()],
    }
}

#[test]
fn wrapping_sampler_for_perf_and_pyspy_returns_sampler() {
    let _ = wrapping_sampler_for(CpuProfilerBackend::Perf);
    let _ = wrapping_sampler_for(CpuProfilerBackend::PySpy);
}

#[test]
#[should_panic(expected = "are arg-augmenting backends")]
fn wrapping_sampler_for_async_profiler_panics_unreachable() {
    let _ = wrapping_sampler_for(CpuProfilerBackend::AsyncProfiler);
}

#[test]
#[should_panic(expected = "are arg-augmenting backends")]
fn wrapping_sampler_for_python_site_sampler_panics_unreachable() {
    let _ = wrapping_sampler_for(CpuProfilerBackend::PythonSiteSampler);
}

#[test]
fn prepare_wrapped_sampler_wrap_fails_propagates_missing_binary_before_any_spawn() {
    let sampler = FakeWrappingSampler {
        outcome: WrapOutcome::WrapFails(CpuProfileError::MissingBinary {
            binary: "perf",
            install_hint: "install linux-tools",
        }),
        wrapped_program: "/usr/bin/perf",
    };

    let result = prepare_wrapped(Box::new(sampler), &sample_request(), "wrap-missing".to_string(), DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    assert!(matches!(result, Err(CpuProfileError::MissingBinary { binary: "perf", .. })));
}

#[cfg(unix)]
#[test]
fn wrapped_finish_collect_fails_returns_finish_error_and_reports_faulted() {
    let recording = start_recording();
    let sampler = FakeWrappingSampler {
        outcome: WrapOutcome::CollectFails,
        wrapped_program: "/usr/bin/perf",
    };
    let PreparedLaunch::Wrapped { session, .. } =
        prepare_wrapped(Box::new(sampler), &sample_request(), "finish-fail".to_string(), DEFAULT_CPU_PROFILE_LIMIT_BYTES).unwrap()
    else {
        panic!("expected Wrapped variant");
    };
    let mut placeholder_child = std::process::Command::new("true")
        .spawn()
        .expect("spawn placeholder child");

    let result = session.finish(&mut placeholder_child);
    let _ = placeholder_child.wait();

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
    assert_eq!(
        recording.recorded.states_for("finish-fail"),
        vec![RunnableState::Stopping, RunnableState::Faulted]
    );
}

#[cfg(unix)]
#[test]
fn wrapped_finish_collect_succeeds_reports_stopped_with_folded_stacks() {
    let recording = start_recording();
    let sampler = FakeWrappingSampler {
        outcome: WrapOutcome::CollectSucceeds,
        wrapped_program: "/usr/bin/perf",
    };
    let PreparedLaunch::Wrapped { session, .. } =
        prepare_wrapped(Box::new(sampler), &sample_request(), "finish-success".to_string(), DEFAULT_CPU_PROFILE_LIMIT_BYTES).unwrap()
    else {
        panic!("expected Wrapped variant");
    };
    let mut placeholder_child = std::process::Command::new("true")
        .spawn()
        .expect("spawn placeholder child");

    let result = session.finish(&mut placeholder_child);
    let _ = placeholder_child.wait();

    assert!(result.is_ok());
    assert_eq!(
        recording.recorded.states_for("finish-success"),
        vec![RunnableState::Stopping, RunnableState::Stopped]
    );
    let folded = recording
        .recorded
        .folded_for("finish-success")
        .expect("folded stacks were not reported");
    assert!(folded.contains("compute"), "got: {folded}");
}

#[cfg(unix)]
#[test]
fn wrapped_sampling_called_reports_started() {
    let recording = start_recording();
    let sampler = FakeWrappingSampler {
        outcome: WrapOutcome::CollectSucceeds,
        wrapped_program: "/usr/bin/perf",
    };
    let PreparedLaunch::Wrapped { session, .. } =
        prepare_wrapped(Box::new(sampler), &sample_request(), "sampling".to_string(), DEFAULT_CPU_PROFILE_LIMIT_BYTES).unwrap()
    else {
        panic!("expected Wrapped variant");
    };

    session.sampling();

    assert_eq!(
        recording.recorded.states_for("sampling"),
        vec![RunnableState::Started]
    );
}

#[cfg(unix)]
#[test]
fn wrapped_finish_no_samples_reports_faulted_with_a_reason() {
    let recording = start_recording();
    let sampler = FakeWrappingSampler {
        outcome: WrapOutcome::CollectFindsNoSamples,
        wrapped_program: "/usr/bin/perf",
    };
    let PreparedLaunch::Wrapped { session, .. } = prepare_wrapped(
        Box::new(sampler),
        &sample_request(),
        "finish-no-samples".to_string(),
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    )
    .unwrap()
    else {
        panic!("expected Wrapped variant");
    };
    let mut placeholder_child = std::process::Command::new("true")
        .spawn()
        .expect("spawn placeholder child");

    let result = session.finish(&mut placeholder_child);
    let _ = placeholder_child.wait();

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
    assert_eq!(
        recording.recorded.states_for("finish-no-samples"),
        vec![RunnableState::Stopping, RunnableState::Faulted]
    );
    assert_eq!(recording.recorded.folded_for("finish-no-samples"), None);
    let reason = recording
        .recorded
        .reason_for("finish-no-samples")
        .expect("a faulted profiler must report why it has no profile");
    assert!(
        reason.contains("no samples were recorded"),
        "the reason must name what went wrong, got: {reason}"
    );
}
