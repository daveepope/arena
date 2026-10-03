use arena_profile::wrapped::{prepare_wrapped, wrapping_sampler_for, WrapState, WrappingSampler};
use arena_profile::{CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch};
use std::path::PathBuf;
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
        _output_path: &std::path::Path,
    ) -> Result<(PathBuf, Vec<String>, WrapState), CpuProfileError> {
        match &self.outcome {
            WrapOutcome::WrapFails(e) => Err(clone_error(e)),
            WrapOutcome::CollectFails | WrapOutcome::CollectSucceeds => {
                let mut args = vec!["--".to_string(), request.program.to_string_lossy().into_owned()];
                args.extend(request.args.iter().cloned());
                Ok((
                    PathBuf::from(self.wrapped_program),
                    args,
                    WrapState::Perf {
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
    ) -> Result<PathBuf, CpuProfileError> {
        match &self.outcome {
            WrapOutcome::CollectFails => Err(CpuProfileError::Finish("collect failed".into())),
            WrapOutcome::CollectSucceeds => Ok(write_sample_folded_stacks("wrapped-collect-succeeds")),
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

fn temp_html_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("arena-profile-wrapped-test-{name}-{}.html", std::process::id()))
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
#[should_panic(expected = "AsyncProfiler is an arg-augmenting backend")]
fn wrapping_sampler_for_async_profiler_panics_unreachable() {
    let _ = wrapping_sampler_for(CpuProfilerBackend::AsyncProfiler);
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

    let result = prepare_wrapped(Box::new(sampler), &sample_request(), temp_html_path("wrap-missing"));

    assert!(matches!(result, Err(CpuProfileError::MissingBinary { binary: "perf", .. })));
}

#[cfg(unix)]
#[test]
fn wrapped_finish_collect_fails_returns_finish_error() {
    let output_path = temp_html_path("wrapped-finish-fail");
    let sampler = FakeWrappingSampler {
        outcome: WrapOutcome::CollectFails,
        wrapped_program: "/usr/bin/perf",
    };
    let PreparedLaunch::Wrapped { session, .. } =
        prepare_wrapped(Box::new(sampler), &sample_request(), output_path).unwrap()
    else {
        panic!("expected Wrapped variant");
    };
    let mut placeholder_child = std::process::Command::new("true")
        .spawn()
        .expect("spawn placeholder child");

    let result = session.finish(&mut placeholder_child);
    let _ = placeholder_child.wait();

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
}

#[cfg(unix)]
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
    let mut placeholder_child = std::process::Command::new("true")
        .spawn()
        .expect("spawn placeholder child");

    let result = session.finish(&mut placeholder_child);
    let _ = placeholder_child.wait();

    assert!(result.is_ok());
    let report = std::fs::read_to_string(&output_path).expect("read rendered report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
}

#[cfg(unix)]
#[test]
fn wrapped_finish_with_hotspots_enabled_renders_hotspots_table() {
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
    let mut placeholder_child = std::process::Command::new("true")
        .spawn()
        .expect("spawn placeholder child");

    let result = session.with_hotspots().finish(&mut placeholder_child);
    let _ = placeholder_child.wait();

    assert!(result.is_ok());
    let report = std::fs::read_to_string(&output_path).expect("read rendered report");
    assert!(report.contains("arena-profile-hotspots"));
    assert!(report.contains("compute"));
    let _ = std::fs::remove_file(&output_path);
}
