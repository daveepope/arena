use crate::render::{render_folded_to_html, RenderError};
use crate::sampler::{self, ArgAugmentingSampler, AugmentState, WrapState, WrappingSampler};
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

pub const FINISH_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuProfilerBackend {
    Perf,
    AsyncProfiler,
    PySpy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownSignal {
    Terminate,
    Kill,
}

#[derive(Debug)]
pub enum CpuProfileError {
    MissingBinary {
        binary: &'static str,
        install_hint: &'static str,
    },
    Spawn(std::io::Error),
    Launch(String),
    Finish(String),
    Render(RenderError),
}

impl std::fmt::Display for CpuProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CpuProfileError::MissingBinary { binary, install_hint } => {
                write!(f, "required profiling tool '{binary}' was not found on PATH ({install_hint})")
            }
            CpuProfileError::Spawn(e) => write!(f, "failed to spawn profiler process: {e}"),
            CpuProfileError::Launch(msg) => write!(f, "failed to prepare profiled launch: {msg}"),
            CpuProfileError::Finish(msg) => write!(f, "failed to finish profiling session: {msg}"),
            CpuProfileError::Render(e) => write!(f, "failed to render flamegraph report: {e}"),
        }
    }
}

impl std::error::Error for CpuProfileError {}

impl From<RenderError> for CpuProfileError {
    fn from(e: RenderError) -> Self {
        CpuProfileError::Render(e)
    }
}

pub struct LaunchRequest {
    pub program: PathBuf,
    pub args: Vec<String>,
}

pub struct WrappedProfileSession {
    sampler: Box<dyn WrappingSampler>,
    state: WrapState,
    output_path: PathBuf,
}

impl WrappedProfileSession {
    pub fn finish(self, wrapping_child: &mut Child) -> Result<(), CpuProfileError> {
        let folded_path = self
            .sampler
            .collect(self.state, wrapping_child, FINISH_TIMEOUT)?;
        render_collected(&folded_path, &self.output_path)
    }
}

pub struct AugmentedProfileSession {
    sampler: Box<dyn ArgAugmentingSampler>,
    state: AugmentState,
    output_path: PathBuf,
}

impl AugmentedProfileSession {
    pub fn finish(self, augmented_process: &mut Child) -> Result<(), CpuProfileError> {
        crate::backend::wait_bounded(augmented_process, FINISH_TIMEOUT)
            .map_err(|e| CpuProfileError::Finish(format!("profiled process did not exit cleanly: {e}")))?;
        let folded_path = self.sampler.collect(self.state, FINISH_TIMEOUT)?;
        render_collected(&folded_path, &self.output_path)
    }
}

pub enum PreparedLaunch {
    Wrapped {
        program: PathBuf,
        args: Vec<String>,
        session: WrappedProfileSession,
    },
    ArgsAugmented {
        args: Vec<String>,
        shutdown_signal: ShutdownSignal,
        session: AugmentedProfileSession,
    },
}

pub fn prepare_cpu_profile(
    backend: CpuProfilerBackend,
    request: LaunchRequest,
    output_path: impl Into<PathBuf>,
) -> Result<PreparedLaunch, CpuProfileError> {
    let output_path = output_path.into();
    match backend {
        CpuProfilerBackend::Perf | CpuProfilerBackend::PySpy => prepare_wrapped(
            sampler::wrapping_sampler_for(backend),
            &request,
            output_path,
        ),
        CpuProfilerBackend::AsyncProfiler => prepare_augmented(
            sampler::augmenting_sampler_for(backend),
            &request,
            output_path,
        ),
    }
}

fn prepare_wrapped(
    sampler: Box<dyn WrappingSampler>,
    request: &LaunchRequest,
    output_path: PathBuf,
) -> Result<PreparedLaunch, CpuProfileError> {
    let (program, args, state) = sampler.wrap(request, &output_path)?;
    Ok(PreparedLaunch::Wrapped {
        program,
        args,
        session: WrappedProfileSession {
            sampler,
            state,
            output_path,
        },
    })
}

fn prepare_augmented(
    sampler: Box<dyn ArgAugmentingSampler>,
    request: &LaunchRequest,
    output_path: PathBuf,
) -> Result<PreparedLaunch, CpuProfileError> {
    let (args, state) = sampler.augment(request, &output_path)?;
    Ok(PreparedLaunch::ArgsAugmented {
        args,
        shutdown_signal: ShutdownSignal::Terminate,
        session: AugmentedProfileSession {
            sampler,
            state,
            output_path,
        },
    })
}

fn render_collected(folded_path: &std::path::Path, output_path: &std::path::Path) -> Result<(), CpuProfileError> {
    let folded_file = std::fs::File::open(folded_path).map_err(|e| {
        CpuProfileError::Finish(format!(
            "failed to read folded stacks at {}: {e}",
            folded_path.display()
        ))
    })?;
    let render_result = render_folded_to_html(folded_file, output_path);
    let _ = std::fs::remove_file(folded_path);
    render_result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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
        let path = std::env::temp_dir().join(format!(
            "arena-profile-test-folded-{name}-{}.folded",
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
        std::env::temp_dir().join(format!("arena-profile-test-{name}-{}.html", std::process::id()))
    }

    fn sample_request() -> LaunchRequest {
        LaunchRequest {
            program: PathBuf::from("/bin/target"),
            args: vec!["a".to_string(), "b".to_string()],
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

        let result = prepare_wrapped(Box::new(sampler), &sample_request(), temp_html_path("wrap-missing"));

        assert!(matches!(result, Err(CpuProfileError::MissingBinary { binary: "perf", .. })));
    }

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

    #[test]
    fn prepare_augmented_sampler_augment_fails_propagates_missing_binary() {
        let sampler = FakeArgAugmentingSampler {
            outcome: AugmentOutcome::AugmentFails,
        };

        let result = prepare_augmented(Box::new(sampler), &sample_request(), temp_html_path("augment-missing"));

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
            prepare_augmented(Box::new(sampler), &sample_request(), temp_html_path("augment-signal")).unwrap()
        else {
            panic!("expected ArgsAugmented variant");
        };

        assert_eq!(shutdown_signal, ShutdownSignal::Terminate);
    }

    #[test]
    fn augmented_finish_collect_fails_returns_finish_error() {
        let output_path = temp_html_path("augmented-finish-fail");
        let sampler = FakeArgAugmentingSampler {
            outcome: AugmentOutcome::CollectFails,
        };
        let PreparedLaunch::ArgsAugmented { session, .. } =
            prepare_augmented(Box::new(sampler), &sample_request(), output_path).unwrap()
        else {
            panic!("expected ArgsAugmented variant");
        };
        let mut placeholder_process = std::process::Command::new("true")
            .spawn()
            .expect("spawn placeholder process");

        let result = session.finish(&mut placeholder_process);
        let _ = placeholder_process.wait();

        assert!(matches!(result, Err(CpuProfileError::Finish(_))));
    }

    #[test]
    fn augmented_finish_collect_succeeds_renders_html_report() {
        let output_path = temp_html_path("augmented-finish-success");
        let sampler = FakeArgAugmentingSampler {
            outcome: AugmentOutcome::CollectSucceeds,
        };
        let PreparedLaunch::ArgsAugmented { session, .. } =
            prepare_augmented(Box::new(sampler), &sample_request(), output_path.clone()).unwrap()
        else {
            panic!("expected ArgsAugmented variant");
        };
        let mut placeholder_process = std::process::Command::new("true")
            .spawn()
            .expect("spawn placeholder process");

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
            "arena-profile-test-missing-folded-{}.folded",
            std::process::id()
        ));

        let result = render_collected(&missing, &temp_html_path("render-collected-missing"));

        assert!(matches!(result, Err(CpuProfileError::Finish(_))));
    }

    #[test]
    fn render_collected_valid_folded_path_removes_folded_file_after_render() {
        let folded_path = write_sample_folded_stacks("render-collected-cleanup");
        let output_path = temp_html_path("render-collected-cleanup");

        let result = render_collected(&folded_path, &output_path);

        assert!(result.is_ok());
        assert!(!folded_path.exists());
        let _ = std::fs::remove_file(&output_path);
    }
}
