use crate::builder::ExecutableComponentBuilder;
use arena::component::RunnableComponent;
use arena::healthcheck::ReadinessCheck;
use arena_profile::{AugmentedProfileSession, PreparedLaunch, ShutdownSignal, WrappedProfileSession};
use async_trait::async_trait;
use futures::FutureExt;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;

enum ActiveCpuProfile {
    Wrapped(WrappedProfileSession),
    ArgAugmented(AugmentedProfileSession, ShutdownSignal),
}

pub struct ExecutableComponent {
    pub(crate) identifier: String,
    pub(crate) children: Option<Vec<Box<dyn RunnableComponent>>>,
    pub(crate) executable_path: Option<PathBuf>,
    pub(crate) env_vars: Vec<(String, String)>,
    pub(crate) runtime_args: Vec<(String, String)>,
    pub(crate) process_handle: Option<Child>,
    pub(crate) stopped: bool,
    pub(crate) readiness_checks: Vec<(Box<dyn ReadinessCheck>, String, u64)>,
    pub(crate) cpu_profile: Option<(arena_profile::CpuProfilerBackend, PathBuf, bool)>,
    active_cpu_profile: Option<ActiveCpuProfile>,
}

impl ExecutableComponent {
    pub(crate) fn new(identifier: String) -> Self {
        ExecutableComponent {
            identifier,
            children: None,
            executable_path: None,
            env_vars: Vec::new(),
            runtime_args: Vec::new(),
            process_handle: None,
            stopped: false,
            readiness_checks: Vec::new(),
            cpu_profile: None,
            active_cpu_profile: None,
        }
    }

    pub fn builder(identifier: impl Into<String>) -> ExecutableComponentBuilder {
        ExecutableComponentBuilder::new(identifier)
    }

    async fn wait_until_ready(&self) {
        if self.readiness_checks.is_empty() {
            return;
        }

        for (check, target, check_timeout_ms) in &self.readiness_checks {
            match check
                .is_ready(&self.identifier, target, *check_timeout_ms)
                .await {
                Ok(()) => {
                    tracing::debug!(
                        component = %self.identifier,
                        readiness_target = %target,
                        "readiness check passed",
                    );
                }
                Err(msg) => {
                    panic!(
                        "{}: readiness check failed for target {}: {}",
                        self.identifier, target, msg
                    );
                }
            }
        }
        tracing::debug!(
            component = %self.identifier,
            "all readiness checks passed",
        );
    }

    fn log_line(identifier: &str, line: &str) {
        if line.contains(" ERROR ") {
            tracing::error!(component = %identifier, "{}", line);
        } else if line.contains(" WARN ") {
            tracing::warn!(component = %identifier, "{}", line);
        } else if line.contains(" DEBUG ") {
            tracing::debug!(component = %identifier, "{}", line);
        } else if line.contains(" TRACE ") {
            tracing::trace!(component = %identifier, "{}", line);
        } else {
            tracing::debug!(component = %identifier, "{}", line);
        }
    }

    fn spawn_output_reader(stream: impl std::io::Read + Send + 'static, identifier: String) {
        thread::spawn(move || {
            let reader = BufReader::new(stream);
            for line in reader.lines() {
                if let Ok(line) = line {
                    Self::log_line(&identifier, &line);
                }
            }
        });
    }

    fn signal_terminate(pid: u32) -> std::io::Result<()> {
        let status = Command::new("kill").args(["-TERM", &pid.to_string()]).status()?;
        if !status.success() {
            return Err(std::io::Error::other(format!(
                "kill -TERM {pid} exited with {status}"
            )));
        }
        Ok(())
    }

    fn on_cpu_profile_finished(&self, result: Result<(), arena_profile::CpuProfileError>) {
        match result {
            Ok(()) => {
                tracing::debug!(
                    component = %self.identifier,
                    phase = "cpu_profile_finish_done",
                    "cpu profile rendered",
                );
                if let Some((_, output_path, true)) = self.cpu_profile.as_ref() {
                    if let Err(e) = arena_profile::open_report(output_path) {
                        tracing::warn!(
                            component = %self.identifier,
                            error = %e,
                            "failed to open cpu profile report",
                        );
                    }
                }
            }
            Err(e) => tracing::error!(
                component = %self.identifier,
                error = %e,
                phase = "cpu_profile_finish_failed",
                "cpu profile finish failed",
            ),
        }
    }

    fn graceful_then_force_kill(child: &mut Child, signal: ShutdownSignal, identifier: &str) {
        match signal {
            ShutdownSignal::Kill => {
                let _ = child.kill();
                let _ = child.wait();
            }
            ShutdownSignal::Terminate => {
                if let Err(e) = Self::signal_terminate(child.id()) {
                    tracing::warn!(component = %identifier, error = %e, "SIGTERM failed, forcing kill");
                    let _ = child.kill();
                    let _ = child.wait();
                    return;
                }
                if let Err(e) = arena_profile::wait_bounded(child, arena_profile::FINISH_TIMEOUT) {
                    tracing::warn!(component = %identifier, error = %e, "graceful shutdown exceeded budget, forced kill");
                }
            }
        }
    }

    fn spawn_process(&mut self) -> Result<(), String> {
        let executable_path = self
            .executable_path
            .as_ref()
            .ok_or_else(|| "executable_path not configured".to_string())?;

        let base_args: Vec<String> = self.runtime_args.iter().map(|(_, v)| v.clone()).collect();

        let (spawn_program, spawn_args, active_profile) = if let Some((backend, output_path, _)) =
            self.cpu_profile.as_ref()
        {
            let request = arena_profile::LaunchRequest {
                program: executable_path.clone(),
                args: base_args,
            };
            let prepared = arena_profile::prepare_cpu_profile(*backend, request, output_path.clone())
                .map_err(|e| format!("cpu profiler preparation failed: {}", e))?;
            match prepared {
                PreparedLaunch::Wrapped { program, args, session } => {
                    (program, args, Some(ActiveCpuProfile::Wrapped(session)))
                }
                PreparedLaunch::ArgsAugmented { args, shutdown_signal, session } => {
                    (executable_path.clone(), args, Some(ActiveCpuProfile::ArgAugmented(session, shutdown_signal)))
                }
            }
        } else {
            (executable_path.clone(), base_args, None)
        };

        tracing::debug!(
            component = %self.identifier,
            spawn_program = ?spawn_program,
            phase = "spawn_begin",
            "spawning child process",
        );

        let mut cmd = Command::new(&spawn_program);

        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        for arg in &spawn_args {
            cmd.arg(arg);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn process: {}", e))?;

        tracing::debug!(
            component = %self.identifier,
            pid = child.id(),
            phase = "spawned",
            "child process spawned",
        );

        self.active_cpu_profile = active_profile;

        if let Some(stdout) = child.stdout.take() {
            Self::spawn_output_reader(stdout, self.identifier.clone());
        }

        if let Some(stderr) = child.stderr.take() {
            Self::spawn_output_reader(stderr, self.identifier.clone());
        }

        self.process_handle = Some(child);

        Ok(())
    }
}

#[async_trait]
impl RunnableComponent for ExecutableComponent {
    async fn start(&mut self) {
        for child in self.children.iter_mut().flatten() {
            child.start().await;
        }

        tracing::debug!(
            component = %self.identifier,
            phase = "start_begin",
            "starting",
        );

        if self.executable_path.is_some() {
            if let Err(e) = self.spawn_process() {
                panic!("{}: spawn failed: {}", self.identifier, e);
            }
        }

        let readiness_result = std::panic::AssertUnwindSafe(self.wait_until_ready())
            .catch_unwind()
            .await;
        if let Err(panic_payload) = readiness_result {
            self.stop().await;
            std::panic::resume_unwind(panic_payload);
        }

        tracing::debug!(
            component = %self.identifier,
            phase = "start_done",
            "started",
        );
    }

    async fn stop(&mut self) {
        if self.stopped {
            return;
        }

        tracing::debug!(
            component = %self.identifier,
            phase = "stop_begin",
            "stopping",
        );

        if let Some(mut child) = self.process_handle.take() {
            match self.active_cpu_profile.take() {
                Some(ActiveCpuProfile::Wrapped(session)) => {
                    tracing::debug!(
                        component = %self.identifier,
                        phase = "cpu_profile_finish_begin",
                        "finishing cpu profile",
                    );
                    let result = session.finish(&mut child);
                    self.on_cpu_profile_finished(result);
                    let _ = child.wait();
                }
                Some(ActiveCpuProfile::ArgAugmented(session, shutdown_signal)) => {
                    tracing::debug!(
                        component = %self.identifier,
                        pid = child.id(),
                        phase = "kill_begin",
                        "stopping child process",
                    );
                    Self::graceful_then_force_kill(&mut child, shutdown_signal, &self.identifier);

                    tracing::debug!(
                        component = %self.identifier,
                        phase = "cpu_profile_finish_begin",
                        "finishing cpu profile",
                    );
                    let result = session.finish();
                    self.on_cpu_profile_finished(result);
                }
                None => {
                    tracing::debug!(
                        component = %self.identifier,
                        pid = child.id(),
                        phase = "kill_begin",
                        "killing child process",
                    );
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }

        tracing::debug!(
            component = %self.identifier,
            phase = "stop_done",
            "stopped",
        );

        for child in self.children.iter_mut().flatten().rev() {
            child.stop().await;
        }

        self.stopped = true;
    }

    fn add_child(&mut self, child: Box<dyn RunnableComponent>) {
        self.children.get_or_insert_with(Vec::new).push(child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_component(identifier: &str) -> ExecutableComponent {
        ExecutableComponent::new(identifier.to_string())
    }

    #[test]
    fn log_line_all_severity_markers_does_not_panic() {
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
    fn signal_terminate_running_child_returns_ok_and_terminates() {
        let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

        let result = ExecutableComponent::signal_terminate(child.id());

        assert!(result.is_ok());
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(child.try_wait().unwrap().is_some());
    }

    #[test]
    fn signal_terminate_already_exited_child_returns_err() {
        let mut child = Command::new("true").spawn().expect("spawn true");
        let _ = child.wait();

        let result = ExecutableComponent::signal_terminate(child.id());

        assert!(result.is_err());
    }

    #[test]
    fn graceful_then_force_kill_kill_signal_kills_child() {
        let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

        ExecutableComponent::graceful_then_force_kill(&mut child, ShutdownSignal::Kill, "test-component");

        assert!(child.try_wait().unwrap().is_some());
    }

    #[test]
    fn graceful_then_force_kill_terminate_signal_stops_child() {
        let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

        ExecutableComponent::graceful_then_force_kill(&mut child, ShutdownSignal::Terminate, "test-component");

        assert!(child.try_wait().unwrap().is_some());
    }

    #[test]
    fn spawn_process_no_executable_path_returns_err() {
        let mut component = new_component("spawn-test");

        let result = component.spawn_process();

        assert_eq!(result, Err("executable_path not configured".to_string()));
    }

    #[test]
    fn spawn_process_program_missing_returns_err() {
        let mut component = new_component("spawn-test");
        component.executable_path = Some(PathBuf::from("/nonexistent/arena-executable-component-fake-binary"));

        let result = component.spawn_process();

        assert!(result.unwrap_err().contains("failed to spawn process"));
    }

    #[test]
    fn on_cpu_profile_finished_err_does_not_panic() {
        let component = new_component("cpu-profile-test");

        component.on_cpu_profile_finished(Err(arena_profile::CpuProfileError::Finish("boom".to_string())));
    }

    #[test]
    fn on_cpu_profile_finished_ok_without_auto_open_does_not_attempt_open() {
        let mut component = new_component("cpu-profile-test");
        component.cpu_profile = Some((
            arena_profile::CpuProfilerBackend::Perf,
            PathBuf::from("/tmp/does-not-matter.html"),
            false,
        ));

        component.on_cpu_profile_finished(Ok(()));
    }
}
