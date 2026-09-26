use arena::lifecycle::message;
use arena::lifecycle::Subject;
use crate::builder::ExecutableComponentBuilder;
use arena::component::RunnableComponent;
use arena::component::Component;
use arena::healthcheck::ReadinessCheck;
use arena_profile::{AugmentedProfileSession, PreparedLaunch, ShutdownSignal, WrappedProfileSession};
use arena::lifecycle::{Fault, RunnableState};
use async_trait::async_trait;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;

enum ActiveCpuProfile {
    Wrapped(WrappedProfileSession),
    ArgAugmented(AugmentedProfileSession, ShutdownSignal),
}

pub(crate) struct CpuProfileConfig {
    pub(crate) backend: arena_profile::CpuProfilerBackend,
    pub(crate) output_path: PathBuf,
    pub(crate) auto_open: bool,
    pub(crate) include_hotspots: bool,
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
    pub(crate) cpu_profile: Option<CpuProfileConfig>,
    active_cpu_profile: Option<ActiveCpuProfile>,
    pub(crate) state: RunnableState,
    pub(crate) faults: Vec<Fault>,
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
            state: RunnableState::NotStarted,
            faults: Vec::new(),
        }
    }

    pub fn builder(identifier: impl Into<String>) -> ExecutableComponentBuilder {
        ExecutableComponentBuilder::new(identifier)
    }

    async fn wait_until_ready(&self) -> Result<(), String> {
        if self.readiness_checks.is_empty() {
            return Ok(());
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
                    return Err(message::readiness_failed_for_target(target, msg));
                }
            }
        }
        tracing::debug!(
            component = %self.identifier,
            "all readiness checks passed",
        );
        Ok(())
    }

    async fn fail(&mut self, message: impl Into<String>, causes: Vec<Fault>) -> Fault {
        let fault = Fault::component(&self.identifier, message).caused_by_all(causes);
        self.faults.push(fault.clone());
        <Self as RunnableComponent>::force_stop(self).await;
        fault
    }

    fn terminate_process(&mut self) {
        let Some(mut child) = self.process_handle.take() else {
            return;
        };
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
                let result = session.finish(&mut child);
                self.on_cpu_profile_finished(result);
                let _ = child.wait();
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

    pub fn log_line(identifier: &str, line: &str) {
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

    pub fn signal_terminate(child: &mut Child) -> std::io::Result<()> {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        let status = Command::new("kill").args(["-TERM", &child.id().to_string()]).status()?;
        if !status.success() {
            if child.try_wait()?.is_some() {
                return Ok(());
            }
            return Err(std::io::Error::other(format!(
                "kill -TERM {} exited with {status}",
                child.id()
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
                if let Some(cfg) = self.cpu_profile.as_ref() {
                    if cfg.auto_open {
                        if let Err(e) = arena_profile::open_report(&cfg.output_path) {
                            tracing::warn!(
                                component = %self.identifier,
                                error = %e,
                                "failed to open cpu profile report",
                            );
                        }
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

    pub fn graceful_then_force_kill(child: &mut Child, signal: ShutdownSignal, identifier: &str) {
        match signal {
            ShutdownSignal::Kill => {
                let _ = child.kill();
                let _ = child.wait();
            }
            ShutdownSignal::Terminate => {
                if let Err(e) = Self::signal_terminate(child) {
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

        let (spawn_program, spawn_args, active_profile) = if let Some(cfg) = self.cpu_profile.as_ref() {
            let request = arena_profile::LaunchRequest {
                program: executable_path.clone(),
                args: base_args,
            };
            let mut prepared = arena_profile::prepare_cpu_profile(cfg.backend, request, cfg.output_path.clone())
                .map_err(|e| format!("cpu profiler preparation failed: {}", e))?;
            if cfg.include_hotspots {
                prepared = prepared.with_hotspots();
            }
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
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn state(&self) -> RunnableState {
        self.state
    }

    fn faults(&self) -> &[Fault] {
        &self.faults
    }

    async fn start(&mut self) -> Result<(), Fault> {
        self.state = RunnableState::Starting;

        let mut child_faults = Vec::new();
        for child in self.children.iter_mut().flatten() {
            if let Err(fault) = arena::component::start_child(child).await {
                child_faults.push(fault);
            }
        }
        if !child_faults.is_empty() {
            return Err(self.fail(message::child_start_failed(Subject::Component), child_faults).await);
        }

        tracing::debug!(
            component = %self.identifier,
            phase = "start_begin",
            "starting",
        );

        if self.executable_path.is_some() {
            if let Err(e) = self.spawn_process() {
                return Err(self.fail(format!("spawn failed: {e}"), Vec::new()).await);
            }
        }

        self.state = RunnableState::ReadinessCheck;
        if let Err(message) = self.wait_until_ready().await {
            return Err(self.fail(message, Vec::new()).await);
        }

        self.state = RunnableState::Started;
        tracing::debug!(
            component = %self.identifier,
            phase = "start_done",
            "started",
        );
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        if self.stopped {
            return Ok(());
        }
        self.state = RunnableState::Stopping;

        tracing::debug!(
            component = %self.identifier,
            phase = "stop_begin",
            "stopping",
        );

        self.terminate_process();

        tracing::debug!(
            component = %self.identifier,
            phase = "stop_done",
            "stopped",
        );

        let mut causes = Vec::new();
        for child in self.children.iter_mut().flatten().rev() {
            if let Err(fault) = arena::component::stop_child(child).await {
                causes.push(fault);
            }
        }

        self.stopped = true;

        if !causes.is_empty() {
            let fault =
                Fault::component(&self.identifier, message::stop_did_not_complete()).caused_by_all(causes);
            self.faults.push(fault.clone());
            self.state = RunnableState::Faulted;
            return Err(fault);
        }

        self.state = RunnableState::Stopped;
        Ok(())
    }

    fn release(&mut self) {
        self.terminate_process();
        self.stopped = true;
        for child in self.children.iter_mut().flatten().rev() {
            arena::component::release_child(child);
        }
        self.state = RunnableState::Stopped;
    }

    async fn force_stop(&mut self) {
        self.terminate_process();
        self.stopped = true;

        for child in self.children.iter_mut().flatten().rev() {
            arena::component::force_stop_child(child).await;
        }

        self.state = RunnableState::Stopped;
    }

    fn add_child(&mut self, child: Box<dyn RunnableComponent>) {
        self.children.get_or_insert_with(Vec::new).push(child);
    }

    fn children(&self) -> &[Component] {
        self.children.as_deref().unwrap_or(&[])
    }

    fn children_mut(&mut self) -> &mut [Component] {
        self.children.as_deref_mut().unwrap_or(&mut [])
    }
}
