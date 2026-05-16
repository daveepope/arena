use crate::builder::ExecutableComponentBuilder;
use arena::component::RunnableComponent;
use arena::healthcheck::ReadinessCheck;
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
    pub(crate) readiness_checks: Vec<(Box<dyn ReadinessCheck>, String)>,
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
        }
    }

    pub fn builder(identifier: impl Into<String>) -> ExecutableComponentBuilder {
        ExecutableComponentBuilder::new(identifier)
    }

    async fn wait_until_ready(&self) {
        if self.readiness_checks.is_empty() {
            return;
        }

        let timeout_ms = 10_000;
        for (check, target) in &self.readiness_checks {
            match check.is_ready(&self.identifier, target, timeout_ms).await {
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

        self.wait_until_ready().await;

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
            tracing::debug!(
                component = %self.identifier,
                pid = child.id(),
                phase = "kill_begin",
                "killing child process",
            );
            let _ = child.kill();
            let _ = child.wait();
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
