use arena::component::RunnableComponent;
use arena::healthcheck::ReadinessCheck;
use arena_executable_component::builder::BuildTool;
use arena_executable_component::executable_component::{
    CpuProfilePreparer, ExecutableComponent, ExternalCpuProfilePreparer,
};
use arena_profiler::profiler::{prepare_augmented, prepare_wrapped};
use arena_profiler::sampler::{ArgAugmentingSampler, AugmentState, WrapState, WrappingSampler};
use arena_profiler::{
    CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch, ShutdownSignal,
};
use async_trait::async_trait;
use futures::FutureExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn enable_debug_logging() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_test_writer()
            .try_init();
    });
}

fn temp_path(name: &str, ext: &str) -> PathBuf {
    let unique_id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "arena-executable-component-test-{name}-{}-{unique_id}.{ext}",
        std::process::id()
    ))
}

fn write_folded_stacks() -> PathBuf {
    let path = temp_path("folded", "folded");
    std::fs::write(&path, "main;handler;compute 42\n").expect("write folded stacks");
    path
}

struct LongRunningWrappingSampler;

impl WrappingSampler for LongRunningWrappingSampler {
    fn wrap(
        &self,
        request: &LaunchRequest,
        _output_path: &Path,
    ) -> Result<(PathBuf, Vec<String>, WrapState), CpuProfileError> {
        let mut args = vec!["-c".to_string(), "sleep 30".to_string()];
        args.push(request.program.to_string_lossy().into_owned());
        Ok((
            PathBuf::from("/bin/sh"),
            args,
            WrapState::Perf { data_path: PathBuf::from("/tmp/unused.data") },
        ))
    }

    fn collect(
        &self,
        _state: WrapState,
        wrapping_child: &mut Child,
        _budget: Duration,
    ) -> Result<PathBuf, CpuProfileError> {
        let _ = wrapping_child.kill();
        let _ = wrapping_child.wait();
        Ok(write_folded_stacks())
    }
}

struct LongRunningAugmentingSampler;

impl ArgAugmentingSampler for LongRunningAugmentingSampler {
    fn augment(
        &self,
        _request: &LaunchRequest,
        _output_path: &Path,
    ) -> Result<(Vec<String>, Vec<(String, String)>, AugmentState), CpuProfileError> {
        Ok((
            vec!["-c".to_string(), "sleep 30".to_string()],
            vec![("ARENA_TEST_AGENT".to_string(), "attached".to_string())],
            AugmentState::AsyncProfiler { folded_path: PathBuf::from("/tmp/unused.collapsed") },
        ))
    }

    fn collect(&self, _state: AugmentState, _budget: Duration) -> Result<PathBuf, CpuProfileError> {
        Ok(write_folded_stacks())
    }
}

enum PreparerOutcome {
    Wrapped,
    ArgAugmented,
    Fails,
}

struct StubCpuProfilePreparer {
    outcome: PreparerOutcome,
}

impl CpuProfilePreparer for StubCpuProfilePreparer {
    fn prepare(
        &self,
        _backend: CpuProfilerBackend,
        request: LaunchRequest,
        output_path: PathBuf,
    ) -> Result<PreparedLaunch, CpuProfileError> {
        match self.outcome {
            PreparerOutcome::Wrapped => {
                prepare_wrapped(Box::new(LongRunningWrappingSampler), &request, output_path)
            }
            PreparerOutcome::ArgAugmented => {
                prepare_augmented(Box::new(LongRunningAugmentingSampler), &request, output_path)
            }
            PreparerOutcome::Fails => Err(CpuProfileError::MissingBinary {
                binary: "perf",
                install_hint: "install linux-tools",
            }),
        }
    }
}

struct PassingReadinessCheck;

#[async_trait]
impl ReadinessCheck for PassingReadinessCheck {
    async fn is_ready(&self, _identifier: &str, _target: &str, _timeout_ms: u64) -> Result<(), String> {
        Ok(())
    }
}

struct FailingReadinessCheck {
    await_pid_file: PathBuf,
}

#[async_trait]
impl ReadinessCheck for FailingReadinessCheck {
    async fn is_ready(&self, _identifier: &str, _target: &str, _timeout_ms: u64) -> Result<(), String> {
        for _ in 0..100 {
            if read_pid(&self.await_pid_file).is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        Err("never became ready".to_string())
    }
}

fn read_pid(pid_file: &Path) -> Option<String> {
    let contents = std::fs::read_to_string(pid_file).ok()?;
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

async fn await_pid(pid_file: &Path) -> String {
    for _ in 0..100 {
        if let Some(pid) = read_pid(pid_file) {
            return pid;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("child process never wrote its pid to {}", pid_file.display());
}

fn profiled_component(outcome: PreparerOutcome, output_path: &Path) -> ExecutableComponent {
    ExecutableComponent::builder("cpu-profile-test")
        .with_build_tool(BuildTool::Cargo)
        .with_executable_path("/bin/sh")
        .with_cpu_profile(output_path)
        .with_cpu_profile_preparer(StubCpuProfilePreparer { outcome })
        .build()
}

fn child_is_running(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .status()
        .expect("run kill -0")
        .success()
}

#[test]
fn log_line_every_severity_marker_does_not_panic() {
    for line in [
        "2024-01-01 ERROR something broke",
        "2024-01-01 WARN heads up",
        "2024-01-01 DEBUG detail",
        "2024-01-01 TRACE fine detail",
        "2024-01-01 INFO normal",
    ] {
        ExecutableComponent::log_line("test-component", line);
    }
}

#[test]
fn signal_terminate_running_child_terminates_it() {
    let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

    let result = ExecutableComponent::signal_terminate(&mut child);

    assert!(result.is_ok());
    std::thread::sleep(Duration::from_millis(100));
    assert!(child.try_wait().unwrap().is_some());
}

#[test]
fn signal_terminate_already_exited_child_returns_ok() {
    let mut child = Command::new("true").spawn().expect("spawn true");
    let _ = child.wait();

    let result = ExecutableComponent::signal_terminate(&mut child);

    assert!(result.is_ok());
}

#[test]
fn stop_child_kill_signal_stops_child() {
    let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

    ExecutableComponent::stop_child(&mut child, ShutdownSignal::Kill, "test-component");

    assert!(child.try_wait().unwrap().is_some());
}

#[test]
fn stop_child_terminate_signal_stops_child() {
    let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

    ExecutableComponent::stop_child(&mut child, ShutdownSignal::Terminate, "test-component");

    assert!(child.try_wait().unwrap().is_some());
}

#[test]
fn spawn_process_no_executable_path_returns_err() {
    let mut component = ExecutableComponent::builder("spawn-test").build();

    let result = component.spawn_process();

    assert_eq!(result, Err("executable_path not configured".to_string()));
}

#[test]
fn spawn_process_missing_program_returns_err() {
    let mut component = ExecutableComponent::builder("spawn-test")
        .with_executable_path("/nonexistent/arena-executable-component-fake-binary")
        .build();

    let result = component.spawn_process();

    assert!(result.unwrap_err().contains("failed to spawn process"));
}

#[test]
fn spawn_process_preparer_fails_returns_err() {
    let output_path = temp_path("preparer-fails", "html");
    let mut component = profiled_component(PreparerOutcome::Fails, &output_path);

    let result = component.spawn_process();

    assert!(result.unwrap_err().contains("cpu profiler preparation failed"));
}

#[test]
fn on_cpu_profile_finished_err_does_not_panic() {
    let component = ExecutableComponent::builder("cpu-profile-test").build();

    component.on_cpu_profile_finished(Err(CpuProfileError::Finish("boom".to_string())));
}

#[test]
fn on_cpu_profile_finished_ok_without_auto_open_does_not_open_report() {
    let output_path = temp_path("no-auto-open", "html");
    let component = profiled_component(PreparerOutcome::Wrapped, &output_path);

    component.on_cpu_profile_finished(Ok(()));
}

#[tokio::test]
async fn stop_no_profile_configured_kills_child_process() {
    let pid_file = temp_path("lifecycle-pid", "pid");
    let pid_file_str = pid_file.to_string_lossy().into_owned();

    let mut component = ExecutableComponent::builder("lifecycle-test")
        .with_executable_path("/bin/sh")
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", format!("echo $$ > {pid_file_str} && sleep 30"))
        .build();

    component.start().await;
    let child_pid = await_pid(&pid_file).await;

    component.stop().await;

    assert!(!child_is_running(&child_pid));
    let _ = std::fs::remove_file(&pid_file);
}

#[tokio::test]
async fn stop_wrapped_profile_configured_renders_html_report() {
    let output_path = temp_path("wrapped-report", "html");
    let mut component = profiled_component(PreparerOutcome::Wrapped, &output_path);

    component.start().await;
    component.stop().await;

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn stop_wrapped_profile_with_hotspots_renders_hotspots_table() {
    let output_path = temp_path("wrapped-hotspots", "html");
    let mut component = ExecutableComponent::builder("cpu-profile-test")
        .with_build_tool(BuildTool::Cargo)
        .with_executable_path("/bin/sh")
        .with_cpu_profile(&output_path)
        .with_hotspots()
        .with_cpu_profile_preparer(StubCpuProfilePreparer { outcome: PreparerOutcome::Wrapped })
        .build();

    component.start().await;
    component.stop().await;

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("arena-profiler-hotspots"));
    assert!(report.contains("compute"));
    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn stop_arg_augmented_profile_configured_renders_html_report() {
    let output_path = temp_path("augmented-report", "html");
    let mut component = profiled_component(PreparerOutcome::ArgAugmented, &output_path);

    component.start().await;
    component.stop().await;

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn stop_called_twice_is_idempotent() {
    let output_path = temp_path("stop-twice", "html");
    let mut component = profiled_component(PreparerOutcome::Wrapped, &output_path);

    component.start().await;
    component.stop().await;
    component.stop().await;

    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn start_readiness_check_fails_stops_child_before_panicking() {
    let pid_file = temp_path("readiness-pid", "pid");
    let pid_file_str = pid_file.to_string_lossy().into_owned();

    let mut component = ExecutableComponent::builder("readiness-test")
        .with_executable_path("/bin/sh")
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", format!("echo $$ > {pid_file_str} && sleep 30"))
        .with_readiness_check(
            FailingReadinessCheck { await_pid_file: pid_file.clone() },
            "target",
        )
        .build();

    let outcome = std::panic::AssertUnwindSafe(component.start()).catch_unwind().await;

    assert!(outcome.is_err(), "expected a failing readiness check to panic");
    let child_pid = read_pid(&pid_file).expect("child process should have written its pid");
    assert!(!child_is_running(&child_pid), "child should be stopped before the panic propagates");
    let _ = std::fs::remove_file(&pid_file);
}

#[test]
fn external_preparer_perf_backend_returns_wrapped_or_missing_binary() {
    let request = LaunchRequest {
        program: PathBuf::from("/bin/true"),
        args: vec![],
    };

    let result = ExternalCpuProfilePreparer.prepare(
        CpuProfilerBackend::Perf,
        request,
        temp_path("external-preparer", "html"),
    );

    match result {
        Ok(PreparedLaunch::Wrapped { .. }) => {}
        Err(CpuProfileError::MissingBinary { .. }) => {}
        Ok(PreparedLaunch::Augmented { .. }) => panic!("expected Wrapped variant"),
        Err(other) => panic!("expected MissingBinary, got {other}"),
    }
}

#[test]
fn add_child_appends_to_children() {
    let mut component = ExecutableComponent::builder("parent").build();
    let child = ExecutableComponent::builder("child").build();

    component.add_child(Box::new(child));
}

#[tokio::test]
async fn start_readiness_check_passes_starts_component() {
    enable_debug_logging();
    let pid_file = temp_path("passing-readiness-pid", "pid");
    let pid_file_str = pid_file.to_string_lossy().into_owned();

    let mut component = ExecutableComponent::builder("readiness-test")
        .with_executable_path("/bin/sh")
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", format!("echo $$ > {pid_file_str} && sleep 30"))
        .with_readiness_check(PassingReadinessCheck, "target")
        .build();

    component.start().await;
    let child_pid = await_pid(&pid_file).await;
    assert!(child_is_running(&child_pid));

    component.stop().await;

    assert!(!child_is_running(&child_pid));
    let _ = std::fs::remove_file(&pid_file);
}

#[tokio::test]
async fn start_child_writing_to_stdout_and_stderr_is_logged() {
    enable_debug_logging();
    let mut component = ExecutableComponent::builder("logging-test")
        .with_executable_path("/bin/sh")
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg(
            "script",
            "echo 'x ERROR broke'; echo 'x WARN careful'; echo 'x DEBUG detail'; \
             echo 'x TRACE fine'; echo plain; echo 'x ERROR from stderr' >&2; sleep 30",
        )
        .build();

    component.start().await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    component.stop().await;
}

#[tokio::test]
async fn start_env_vars_configured_passes_them_to_the_child() {
    let env_file = temp_path("env-capture", "txt");
    let env_file_str = env_file.to_string_lossy().into_owned();

    let mut component = ExecutableComponent::builder("env-test")
        .with_executable_path("/bin/sh")
        .with_env_var("ARENA_TEST_VALUE", "captured")
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", format!("echo \"$ARENA_TEST_VALUE\" > {env_file_str}; sleep 30"))
        .build();

    component.start().await;
    for _ in 0..100 {
        if read_pid(&env_file).is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    component.stop().await;

    let captured = std::fs::read_to_string(&env_file).expect("read captured env value");
    assert_eq!(captured.trim(), "captured");
    let _ = std::fs::remove_file(&env_file);
}

#[tokio::test]
async fn start_profile_env_vars_configured_passes_them_to_the_child() {
    enable_debug_logging();
    let env_file = temp_path("profile-env-capture", "txt");
    let env_file_str = env_file.to_string_lossy().into_owned();
    let output_path = temp_path("profile-env", "html");

    let mut component = ExecutableComponent::builder("profile-env-test")
        .with_build_tool(BuildTool::Dotnet)
        .with_executable_path("/bin/sh")
        .with_cpu_profile(&output_path)
        .with_cpu_profile_preparer(StubCpuProfilePreparer {
            outcome: PreparerOutcome::ArgAugmented,
        })
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", format!("echo \"$DOTNET_PerfMapEnabled\" > {env_file_str}; sleep 30"))
        .build();

    component.start().await;
    component.stop().await;

    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_file(&env_file);
}
