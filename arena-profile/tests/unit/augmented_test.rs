#[path = "../support/recorded_profiles.rs"]
mod recorded_profiles;

use arena::lifecycle::RunnableState;
use arena_profile::augmented::{augmenting_sampler_for, prepare_augmented, ArgAugmentingSampler, AugmentState, Augmentation};
use arena_profile::DEFAULT_CPU_PROFILE_LIMIT_BYTES;
use arena_profile::{
    CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch, ProfileEnvVar, ShutdownSignal,
};
use recorded_profiles::start_recording;
use std::path::PathBuf;
use std::time::Duration;

enum AugmentOutcome {
    AugmentFails,
    CollectFails,
    CollectSucceeds,
    CollectFindsNoSamples,
}

struct FakeArgAugmentingSampler {
    outcome: AugmentOutcome,
}

impl ArgAugmentingSampler for FakeArgAugmentingSampler {
    fn augment(&self, request: &LaunchRequest) -> Result<Augmentation, CpuProfileError> {
        match &self.outcome {
            AugmentOutcome::AugmentFails => Err(CpuProfileError::MissingBinary {
                binary: "libasyncProfiler.so",
                install_hint: "install async-profiler",
            }),
            AugmentOutcome::CollectFails
            | AugmentOutcome::CollectSucceeds
            | AugmentOutcome::CollectFindsNoSamples => {
                let mut args = request.args.clone();
                args.push(request.program.to_string_lossy().into_owned());
                Ok(Augmentation {
                    args,
                    env: vec![ProfileEnvVar::space_joined("JAVA_TOOL_OPTIONS", "-agentpath:probe")],
                    state: AugmentState::AsyncProfiler {
                        scratch: arena_profile::backend::ScratchDir::create("arena-profile-augmented-test")
                            .expect("scratch"),
                        folded_path: PathBuf::from("/tmp/unused.collapsed"),
                    },
                })
            }
        }
    }

    fn collect(
        &self,
        _state: AugmentState,
        _budget: Duration,
        _profiled_pid: u32,
        limit_bytes: usize,
    ) -> Result<String, CpuProfileError> {
        match &self.outcome {
            AugmentOutcome::CollectFails => Err(CpuProfileError::Finish("collect failed".into())),
            AugmentOutcome::CollectSucceeds => {
                let path = write_sample_folded_stacks("augmented-collect-succeeds");
                arena_profile::profiler::read_collected(&path, limit_bytes)
            }
            AugmentOutcome::CollectFindsNoSamples => {
                Err(CpuProfileError::Finish("no samples were recorded".into()))
            }
            AugmentOutcome::AugmentFails => unreachable!("collect() called after augment() failed"),
        }
    }
}

fn async_profiler_scratch() -> arena_profile::backend::ScratchDir {
    arena_profile::backend::ScratchDir::create("arena-profile-augmented-test").expect("scratch")
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
    let _ = augmenting_sampler_for(CpuProfilerBackend::AsyncProfiler, DEFAULT_CPU_PROFILE_LIMIT_BYTES);
}

#[test]
#[should_panic(expected = "Perf/PySpy/DotnetTrace are wrapping backends")]
fn augmenting_sampler_for_perf_panics_unreachable() {
    let _ = augmenting_sampler_for(CpuProfilerBackend::Perf, DEFAULT_CPU_PROFILE_LIMIT_BYTES);
}

#[test]
#[should_panic(expected = "Perf/PySpy/DotnetTrace are wrapping backends")]
fn augmenting_sampler_for_pyspy_panics_unreachable() {
    let _ = augmenting_sampler_for(CpuProfilerBackend::PySpy, DEFAULT_CPU_PROFILE_LIMIT_BYTES);
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

fn async_profiler_output(name: &str, contents: &str) -> PathBuf {
    let pattern = std::env::temp_dir().join(format!(
        "arena-profile-async-profiler-test-{name}-{}.collapsed",
        std::process::id()
    ));
    let written = pattern.with_extension(format!("{}.collapsed", std::process::id()));
    std::fs::write(&written, contents).expect("write async-profiler output");
    pattern
}

fn async_profiler_written(pattern: &PathBuf) -> PathBuf {
    pattern.with_extension(format!("{}.collapsed", std::process::id()))
}

#[test]
fn async_profiler_collect_empty_output_returns_finish_error() {
    let folded_path = async_profiler_output("empty", "");
    let sampler = augmenting_sampler_for(CpuProfilerBackend::AsyncProfiler, DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    let result = sampler.collect(
        AugmentState::AsyncProfiler {
            scratch: async_profiler_scratch(),
            folded_path: folded_path.clone(),
        },
        Duration::from_secs(5),
        std::process::id(),
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    );

    match result {
        Err(CpuProfileError::Finish(message)) => assert!(
            message.contains("recorded no samples"),
            "the failure must say no samples were recorded, got: {message}"
        ),
        other => panic!("an empty profile must fault, got {other:?}"),
    }
    let _ = std::fs::remove_file(async_profiler_written(&folded_path));
}

#[test]
fn async_profiler_collect_sampled_output_returns_the_folded_stacks() {
    let folded_path = async_profiler_output("sampled", "main;run;compute 7\n");
    let sampler = augmenting_sampler_for(CpuProfilerBackend::AsyncProfiler, DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    let result = sampler.collect(
        AugmentState::AsyncProfiler {
            scratch: async_profiler_scratch(),
            folded_path: folded_path.clone(),
        },
        Duration::from_secs(5),
        std::process::id(),
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    );

    assert_eq!(result.unwrap(), "main;run;compute 7\n".to_string());
    let _ = std::fs::remove_file(async_profiler_written(&folded_path));
}

#[test]
fn async_profiler_collect_missing_output_returns_finish_error() {
    let folded_path = std::env::temp_dir().join(format!(
        "arena-profile-async-profiler-test-absent-{}.collapsed",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&folded_path);
    let sampler = augmenting_sampler_for(CpuProfilerBackend::AsyncProfiler, DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    let result = sampler.collect(
        AugmentState::AsyncProfiler { scratch: async_profiler_scratch(), folded_path },
        Duration::from_secs(5),
        std::process::id(),
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    );

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
}

#[test]
fn augmented_finish_no_samples_reports_faulted_with_a_reason() {
    let recording = start_recording();
    let sampler = FakeArgAugmentingSampler { outcome: AugmentOutcome::CollectFindsNoSamples };
    let PreparedLaunch::ArgsAugmented { session, .. } = prepare_augmented(
        Box::new(sampler),
        &sample_request(),
        "augmented-no-samples".to_string(),
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    )
    .unwrap()
    else {
        panic!("expected ArgsAugmented variant");
    };
    let mut placeholder_child = std::process::Command::new("true")
        .spawn()
        .expect("spawn placeholder child");

    let result = session.finish(&mut placeholder_child);
    let _ = placeholder_child.wait();

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
    assert_eq!(
        recording.recorded.states_for("augmented-no-samples"),
        vec![RunnableState::Stopping, RunnableState::Faulted]
    );
    assert_eq!(recording.recorded.folded_for("augmented-no-samples"), None);
    let reason = recording
        .recorded
        .reason_for("augmented-no-samples")
        .expect("a faulted profiler must report why it has no profile");
    assert!(
        reason.contains("no samples were recorded"),
        "the reason must name what went wrong, got: {reason}"
    );
}

#[test]
fn async_profiler_collect_output_of_another_process_is_not_accepted() {
    let pattern = async_profiler_output("other-process", "main;run;compute 7\n");
    let sampler = augmenting_sampler_for(CpuProfilerBackend::AsyncProfiler, DEFAULT_CPU_PROFILE_LIMIT_BYTES);

    let result = sampler.collect(
        AugmentState::AsyncProfiler {
            scratch: async_profiler_scratch(),
            folded_path: pattern.clone(),
        },
        Duration::from_secs(5),
        std::process::id() + 1,
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    );

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
    let _ = std::fs::remove_file(async_profiler_written(&pattern));
}
