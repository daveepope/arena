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
