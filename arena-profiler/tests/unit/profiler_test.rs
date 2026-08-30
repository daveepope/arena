use arena_profiler::profiler::{prepare_augmented, prepare_wrapped, render_collected};
use arena_profiler::sampler::{ArgAugmentingSampler, AugmentState, WrapState, WrappingSampler};
use arena_profiler::{
    prepare_cpu_profile, CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch,
    RenderError, ShutdownSignal,
};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

enum WrapOutcome {
    WrapFails(CpuProfileError),
    CollectFails,
    CollectSucceeds,
}

struct FakeWrappingSampler {
    outcome: WrapOutcome,
    wrapped_program: &'static str,
}

impl WrappingSampler for FakeWrappingSampler {
    fn wrap(
        &self,
        request: &LaunchRequest,
        _output_path: &Path,
    ) -> Result<(PathBuf, Vec<String>, WrapState), CpuProfileError> {
        match &self.outcome {
            WrapOutcome::WrapFails(e) => Err(clone_error(e)),
            WrapOutcome::CollectFails | WrapOutcome::CollectSucceeds => {
                let mut args =
                    vec!["--".to_string(), request.program.to_string_lossy().into_owned()];
                args.extend(request.args.iter().cloned());
                Ok((
                    PathBuf::from(self.wrapped_program),
                    args,
                    WrapState::Perf { data_path: PathBuf::from("/tmp/unused.data") },
                ))
            }
        }
    }

    fn collect(
        &self,
        _state: WrapState,
        _wrapping_child: &mut Child,
        _budget: Duration,
    ) -> Result<PathBuf, CpuProfileError> {
        match &self.outcome {
            WrapOutcome::CollectFails => Err(CpuProfileError::Finish("collect failed".into())),
            WrapOutcome::CollectSucceeds => {
                Ok(write_sample_folded_stacks("wrapped-collect-succeeds"))
            }
            WrapOutcome::WrapFails(_) => unreachable!("collect() called after wrap() failed"),
        }
    }
}

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
        _output_path: &Path,
    ) -> Result<(Vec<String>, Vec<(String, String)>, AugmentState), CpuProfileError> {
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
                    vec![("JAVA_TOOL_OPTIONS".to_string(), "-agentpath:fake".to_string())],
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
            AugmentOutcome::CollectSucceeds => {
                Ok(write_sample_folded_stacks("augmented-collect-succeeds"))
            }
            AugmentOutcome::AugmentFails => unreachable!("collect() called after augment() failed"),
        }
    }
}

fn write_sample_folded_stacks(name: &str) -> PathBuf {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let unique_id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "arena-profiler-test-folded-{name}-{}-{unique_id}.folded",
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

fn temp_html_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("arena-profiler-test-{name}-{}.html", std::process::id()))
}

fn sample_request() -> LaunchRequest {
    LaunchRequest {
        program: PathBuf::from("/bin/target"),
        args: vec!["a".to_string(), "b".to_string()],
    }
}

fn spawn_placeholder_child() -> Child {
    std::process::Command::new("true").spawn().expect("spawn placeholder child")
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
        Ok(PreparedLaunch::Augmented { .. }) => panic!("expected Wrapped variant"),
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
        Ok(PreparedLaunch::Augmented { .. }) => {}
        Err(CpuProfileError::MissingBinary { .. }) => {}
        Ok(PreparedLaunch::Wrapped { .. }) => panic!("expected Augmented variant"),
        Err(other) => panic!("expected MissingBinary, got {other}"),
    }
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

    let result =
        prepare_wrapped(Box::new(sampler), &sample_request(), temp_html_path("wrap-missing"));

    assert!(matches!(result, Err(CpuProfileError::MissingBinary { binary: "perf", .. })));
}

#[test]
fn wrapped_finish_collect_fails_returns_finish_error() {
    let sampler = FakeWrappingSampler {
        outcome: WrapOutcome::CollectFails,
        wrapped_program: "/usr/bin/perf",
    };
    let PreparedLaunch::Wrapped { session, .. } = prepare_wrapped(
        Box::new(sampler),
        &sample_request(),
        temp_html_path("wrapped-finish-fail"),
    )
    .unwrap() else {
        panic!("expected Wrapped variant");
    };
    let mut placeholder_child = spawn_placeholder_child();

    let result = session.finish(&mut placeholder_child);
    let _ = placeholder_child.wait();

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
}

#[test]
fn wrapped_finish_collect_succeeds_renders_html_report() {
    let output_path = temp_html_path("wrapped-finish-success");
    let sampler = FakeWrappingSampler {
        outcome: WrapOutcome::CollectSucceeds,
        wrapped_program: "/usr/bin/perf",
    };
    let PreparedLaunch::Wrapped { session, .. } =
        prepare_wrapped(Box::new(sampler), &sample_request(), output_path.clone()).unwrap()
    else {
        panic!("expected Wrapped variant");
    };
    let mut placeholder_child = spawn_placeholder_child();

    let result = session.finish(&mut placeholder_child);
    let _ = placeholder_child.wait();

    assert!(result.is_ok());
    let report = std::fs::read_to_string(&output_path).expect("read rendered report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn wrapped_finish_with_hotspots_renders_hotspots_table() {
    let output_path = temp_html_path("wrapped-finish-hotspots");
    let sampler = FakeWrappingSampler {
        outcome: WrapOutcome::CollectSucceeds,
        wrapped_program: "/usr/bin/perf",
    };
    let PreparedLaunch::Wrapped { session, .. } =
        prepare_wrapped(Box::new(sampler), &sample_request(), output_path.clone()).unwrap()
    else {
        panic!("expected Wrapped variant");
    };
    let mut placeholder_child = spawn_placeholder_child();

    let result = session.with_hotspots().finish(&mut placeholder_child);
    let _ = placeholder_child.wait();

    assert!(result.is_ok());
    let report = std::fs::read_to_string(&output_path).expect("read rendered report");
    assert!(report.contains("arena-profiler-hotspots"));
    assert!(report.contains("compute"));
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn prepare_augmented_sampler_augment_fails_propagates_missing_binary() {
    let sampler = FakeArgAugmentingSampler { outcome: AugmentOutcome::AugmentFails };

    let result =
        prepare_augmented(Box::new(sampler), &sample_request(), temp_html_path("augment-missing"));

    assert!(matches!(
        result,
        Err(CpuProfileError::MissingBinary { binary: "libasyncProfiler.so", .. })
    ));
}

#[test]
fn prepare_augmented_sampler_augment_succeeds_defaults_shutdown_signal_to_terminate() {
    let sampler = FakeArgAugmentingSampler { outcome: AugmentOutcome::CollectFails };

    let PreparedLaunch::Augmented { shutdown_signal, .. } =
        prepare_augmented(Box::new(sampler), &sample_request(), temp_html_path("augment-signal"))
            .unwrap()
    else {
        panic!("expected Augmented variant");
    };

    assert_eq!(shutdown_signal, ShutdownSignal::Terminate);
}

#[test]
fn augmented_finish_collect_fails_returns_finish_error() {
    let sampler = FakeArgAugmentingSampler { outcome: AugmentOutcome::CollectFails };
    let PreparedLaunch::Augmented { session, .. } = prepare_augmented(
        Box::new(sampler),
        &sample_request(),
        temp_html_path("augmented-finish-fail"),
    )
    .unwrap() else {
        panic!("expected Augmented variant");
    };
    let mut placeholder_process = spawn_placeholder_child();

    let result = session.finish(&mut placeholder_process);
    let _ = placeholder_process.wait();

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
}

#[test]
fn augmented_finish_collect_succeeds_renders_html_report() {
    let output_path = temp_html_path("augmented-finish-success");
    let sampler = FakeArgAugmentingSampler { outcome: AugmentOutcome::CollectSucceeds };
    let PreparedLaunch::Augmented { session, .. } =
        prepare_augmented(Box::new(sampler), &sample_request(), output_path.clone()).unwrap()
    else {
        panic!("expected Augmented variant");
    };
    let mut placeholder_process = spawn_placeholder_child();

    let result = session.finish(&mut placeholder_process);
    let _ = placeholder_process.wait();

    assert!(result.is_ok());
    let report = std::fs::read_to_string(&output_path).expect("read rendered report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn render_collected_missing_folded_path_returns_finish_error() {
    let missing = std::env::temp_dir().join(format!(
        "arena-profiler-test-missing-folded-{}.folded",
        std::process::id()
    ));

    let result = render_collected(&missing, &temp_html_path("render-collected-missing"), false);

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
}

#[test]
fn render_collected_valid_folded_path_removes_folded_file_after_render() {
    let folded_path = write_sample_folded_stacks("render-collected-cleanup");
    let output_path = temp_html_path("render-collected-cleanup");

    let result = render_collected(&folded_path, &output_path, false);

    assert!(result.is_ok());
    assert!(!folded_path.exists());
    let _ = std::fs::remove_file(&output_path);
}

#[test]
fn prepare_augmented_sampler_augment_succeeds_returns_env_vars() {
    let sampler = FakeArgAugmentingSampler { outcome: AugmentOutcome::CollectFails };

    let PreparedLaunch::Augmented { env_vars, .. } =
        prepare_augmented(Box::new(sampler), &sample_request(), temp_html_path("augment-env"))
            .unwrap()
    else {
        panic!("expected Augmented variant");
    };

    assert_eq!(
        env_vars,
        vec![("JAVA_TOOL_OPTIONS".to_string(), "-agentpath:fake".to_string())]
    );
}
