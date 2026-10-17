#[path = "../support/recorded_profiles.rs"]
mod recorded_profiles;

use arena::lifecycle::RunnableState;
use arena_profile::augmented::{augmenting_sampler_for, prepare_augmented, ArgAugmentingSampler, AugmentState};
use arena_profile::DEFAULT_CPU_PROFILE_LIMIT_BYTES;
use arena_profile::{CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch, ShutdownSignal};
use recorded_profiles::start_recording;
use std::path::PathBuf;
use std::time::Duration;

enum AugmentOutcome {
    AugmentFails,
    CollectFails,
    CollectSucceeds,
}

struct FakeArgAugmentingSampler {
    outcome: AugmentOutcome,
}

impl ArgAugmentingSampler for FakeArgAugmentingSampler {
    fn augment(
        &self,
        request: &LaunchRequest,
    ) -> Result<(Vec<String>, AugmentState), CpuProfileError> {
        match &self.outcome {
            AugmentOutcome::AugmentFails => Err(CpuProfileError::MissingBinary {
                binary: "libasyncProfiler.so",
                install_hint: "install async-profiler",
            }),
            AugmentOutcome::CollectFails | AugmentOutcome::CollectSucceeds => {
                let mut args = request.args.clone();
                args.push(request.program.to_string_lossy().into_owned());
                Ok((
                    args,
                    AugmentState::AsyncProfiler {
                        folded_path: PathBuf::from("/tmp/unused.collapsed"),
                    },
                ))
            }
        }
    }

    fn collect(&self, _state: AugmentState, _budget: Duration) -> Result<PathBuf, CpuProfileError> {
        match &self.outcome {
            AugmentOutcome::CollectFails => Err(CpuProfileError::Finish("collect failed".into())),
            AugmentOutcome::CollectSucceeds => Ok(write_sample_folded_stacks("augmented-collect-succeeds")),
            AugmentOutcome::AugmentFails => unreachable!("collect() called after augment() failed"),
        }
    }
}

fn write_sample_folded_stacks(name: &str) -> PathBuf {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let unique_id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "arena-profile-augmented-test-folded-{name}-{}-{unique_id}.folded",
        std::process::id()
    ));
    std::fs::write(&path, "main;handler;compute 42\n").expect("write sample folded stacks");
    path
}

fn sample_request() -> LaunchRequest {
    LaunchRequest {
        program: PathBuf::from("/bin/target"),
        args: vec!["a".to_string(), "b".to_string()],
    }
}

#[test]
fn augmenting_sampler_for_async_profiler_returns_sampler() {
    let _ = augmenting_sampler_for(CpuProfilerBackend::AsyncProfiler);
}

#[test]
#[should_panic(expected = "Perf/PySpy/DotnetTrace are wrapping backends")]
fn augmenting_sampler_for_perf_panics_unreachable() {
    let _ = augmenting_sampler_for(CpuProfilerBackend::Perf);
}

#[test]
#[should_panic(expected = "Perf/PySpy/DotnetTrace are wrapping backends")]
fn augmenting_sampler_for_pyspy_panics_unreachable() {
    let _ = augmenting_sampler_for(CpuProfilerBackend::PySpy);
}

#[test]
fn prepare_augmented_sampler_augment_fails_propagates_missing_binary() {
    let sampler = FakeArgAugmentingSampler {
        outcome: AugmentOutcome::AugmentFails,
    };

    let result = prepare_augmented(Box::new(sampler), &sample_request(), "augment-missing".to_string(), DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    assert!(matches!(
        result,
        Err(CpuProfileError::MissingBinary { binary: "libasyncProfiler.so", .. })
    ));
}

#[test]
fn prepare_augmented_sampler_augment_succeeds_defaults_shutdown_signal_to_terminate() {
    let sampler = FakeArgAugmentingSampler {
        outcome: AugmentOutcome::CollectFails,
    };

    let PreparedLaunch::ArgsAugmented { shutdown_signal, .. } =
        prepare_augmented(Box::new(sampler), &sample_request(), "augment-signal".to_string(), DEFAULT_CPU_PROFILE_LIMIT_BYTES).unwrap()
    else {
        panic!("expected ArgsAugmented variant");
    };

    assert_eq!(shutdown_signal, ShutdownSignal::Terminate);
}

#[cfg(unix)]
#[test]
fn augmented_finish_collect_fails_returns_finish_error_and_reports_faulted() {
    let recording = start_recording();
    let sampler = FakeArgAugmentingSampler {
        outcome: AugmentOutcome::CollectFails,
    };
    let PreparedLaunch::ArgsAugmented { session, .. } =
        prepare_augmented(Box::new(sampler), &sample_request(), "finish-fail".to_string(), DEFAULT_CPU_PROFILE_LIMIT_BYTES).unwrap()
    else {
        panic!("expected ArgsAugmented variant");
    };
    let mut placeholder_process = std::process::Command::new("true")
        .spawn()
        .expect("spawn placeholder process");

    let result = session.finish(&mut placeholder_process);
    let _ = placeholder_process.wait();

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
    assert_eq!(
        recording.recorded.states_for("finish-fail"),
        vec![RunnableState::Stopping, RunnableState::Faulted]
    );
}

#[cfg(unix)]
#[test]
fn augmented_finish_collect_succeeds_reports_stopped_with_folded_stacks() {
    let recording = start_recording();
    let sampler = FakeArgAugmentingSampler {
        outcome: AugmentOutcome::CollectSucceeds,
    };
    let PreparedLaunch::ArgsAugmented { session, .. } =
        prepare_augmented(Box::new(sampler), &sample_request(), "finish-success".to_string(), DEFAULT_CPU_PROFILE_LIMIT_BYTES).unwrap()
    else {
        panic!("expected ArgsAugmented variant");
    };
    let mut placeholder_process = std::process::Command::new("true")
        .spawn()
        .expect("spawn placeholder process");

    let result = session.finish(&mut placeholder_process);
    let _ = placeholder_process.wait();

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
