use arena::lifecycle::message;
use arena::lifecycle::Subject;
use crate::builder::ExecutableComponentBuilder;
use arena::component::RunnableComponent;
use arena::component::Component;
use arena::healthcheck::ReadinessCheck;
use arena::lifecycle::{Fault, RunnableState};
use async_trait::async_trait;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;

pub struct ExecutableComponent {
    pub(crate) identifier: String,
    pub(crate) children: Option<Vec<Box<dyn RunnableComponent>>>,
    pub(crate) executable_path: Option<PathBuf>,
    pub(crate) env_vars: Vec<(String, String)>,
    pub(crate) runtime_args: Vec<(String, String)>,
    pub(crate) process_handle: Option<Child>,
    pub(crate) stopped: bool,
    pub(crate) readiness_checks: Vec<(Box<dyn ReadinessCheck>, String, u64)>,
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
        if let Some(mut child) = self.process_handle.take() {
            tracing::debug!(
                component = %self.identifier,
                pid = child.id(),
                phase = "terminate_begin",
                "terminating child process",
            );
            let _ = child.kill();
            let _ = child.wait();
        }
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

    fn spawn_process(&mut self) -> Result<(), String> {
        let executable_path = self
            .executable_path
            .as_ref()
            .ok_or_else(|| "executable_path not configured".to_string())?;

        tracing::debug!(
            component = %self.identifier,
            executable_path = ?executable_path,
            phase = "spawn_begin",
            "spawning child process",
        );

        let mut cmd = Command::new(executable_path);

        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        for (_key, value) in &self.runtime_args {
            cmd.arg(value);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn process: {}", e))?;

        let pid = child.id();
        tracing::debug!(
            component = %self.identifier,
            pid,
            phase = "spawned",
            "child process spawned",
        );

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
