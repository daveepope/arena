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
    pub fn finish(self) -> Result<(), CpuProfileError> {
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
    render_folded_to_html(folded_file, output_path)?;
    let _ = std::fs::remove_file(folded_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    enum WrapOutcome {
        WrapFails(CpuProfileError),
        CollectSucceeds(&'static str),
        CollectFails,
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
                _ => {
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
                WrapOutcome::CollectFails => {
                    Err(CpuProfileError::Finish("collect failed".into()))
                }
                WrapOutcome::CollectSucceeds(folded_output) => {
                    let path = std::env::temp_dir().join(format!(
                        "arena-profile-fake-wrap-test-{}-{:p}.folded",
                        std::process::id(),
                        self
                    ));
                    std::fs::write(&path, folded_output).expect("write fake folded stacks");
                    Ok(path)
                }
                WrapOutcome::WrapFails(_) => unreachable!("collect() called after wrap() failed"),
            }
        }
    }

    enum AugmentOutcome {
        AugmentSucceeds(&'static str),
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
            let mut args = vec!["-agentpath:fake.so=start".to_string()];
            args.extend(request.args.iter().cloned());
            Ok((
                args,
                AugmentState::AsyncProfiler {
                    folded_path: PathBuf::from("/tmp/unused.collapsed"),
                },
            ))
        }

        fn collect(&self, _state: AugmentState, _budget: Duration) -> Result<PathBuf, CpuProfileError> {
            let AugmentOutcome::AugmentSucceeds(folded_output) = &self.outcome;
            let path = std::env::temp_dir().join(format!(
                "arena-profile-fake-augment-test-{}-{:p}.folded",
                std::process::id(),
                self
            ));
            std::fs::write(&path, folded_output).expect("write fake folded stacks");
            Ok(path)
        }
    }

    fn clone_error(e: &CpuProfileError) -> CpuProfileError {
        match e {
            CpuProfileError::MissingBinary { binary, install_hint } => {
                CpuProfileError::MissingBinary { binary, install_hint }
            }
            CpuProfileError::Launch(msg) => CpuProfileError::Launch(msg.clone()),
            CpuProfileError::Finish(msg) => CpuProfileError::Finish(msg.clone()),
            _ => CpuProfileError::Launch("unexpected fake error clone".into()),
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
    fn prepare_wrapped_sampler_succeeds_returns_wrapped_launch_with_program_and_args_in_order() {
        let sampler = FakeWrappingSampler {
            outcome: WrapOutcome::CollectSucceeds("main;foo 1\n"),
            wrapped_program: "/usr/bin/perf",
        };

        let result = prepare_wrapped(Box::new(sampler), &sample_request(), temp_html_path("wrap-ok"));

        match result.unwrap() {
            PreparedLaunch::Wrapped { program, args, .. } => {
                assert_eq!(program, PathBuf::from("/usr/bin/perf"));
                assert_eq!(args, vec!["--", "/bin/target", "a", "b"]);
            }
            PreparedLaunch::ArgsAugmented { .. } => panic!("expected Wrapped variant"),
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
    fn wrapped_finish_collect_succeeds_renders_html_report() {
        let output_path = temp_html_path("wrapped-finish-ok");
        let sampler = FakeWrappingSampler {
            outcome: WrapOutcome::CollectSucceeds("main;foo 3\nmain;bar 1\n"),
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
        session.finish(&mut placeholder_child).unwrap();
        let _ = placeholder_child.wait();

        let report = std::fs::read_to_string(&output_path).unwrap();
        assert!(report.contains("<html"));
        assert!(report.contains("<svg"));
        let _ = std::fs::remove_file(&output_path);
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
    fn prepare_augmented_sampler_prepends_agent_arg_ahead_of_original_args_and_requests_terminate() {
        let sampler = FakeArgAugmentingSampler {
            outcome: AugmentOutcome::AugmentSucceeds("main;foo 1\n"),
        };
        let request = LaunchRequest {
            program: PathBuf::from("java"),
            args: vec!["-jar".to_string(), "app.jar".to_string()],
        };

        let result = prepare_augmented(Box::new(sampler), &request, temp_html_path("augment-ok"));

        match result.unwrap() {
            PreparedLaunch::ArgsAugmented { args, shutdown_signal, .. } => {
                assert_eq!(args, vec!["-agentpath:fake.so=start", "-jar", "app.jar"]);
                assert_eq!(shutdown_signal, ShutdownSignal::Terminate);
            }
            PreparedLaunch::Wrapped { .. } => panic!("expected ArgsAugmented variant"),
        }
    }

    #[test]
    fn augmented_finish_collect_succeeds_renders_html_report() {
        let output_path = temp_html_path("augmented-finish-ok");
        let sampler = FakeArgAugmentingSampler {
            outcome: AugmentOutcome::AugmentSucceeds("main;foo 3\nmain;bar 1\n"),
        };
        let request = LaunchRequest {
            program: PathBuf::from("java"),
            args: vec![],
        };
        let PreparedLaunch::ArgsAugmented { session, .. } =
            prepare_augmented(Box::new(sampler), &request, output_path.clone()).unwrap()
        else {
            panic!("expected ArgsAugmented variant");
        };

        session.finish().unwrap();

        let report = std::fs::read_to_string(&output_path).unwrap();
        assert!(report.contains("<html"));
        assert!(report.contains("<svg"));
        let _ = std::fs::remove_file(&output_path);
    }
}
