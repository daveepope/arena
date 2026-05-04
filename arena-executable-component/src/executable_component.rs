use async_trait::async_trait;
use arena::component::RunnableComponent;
use arena::healthcheck::ReadinessCheck;
use crate::builder::ExecutableComponentBuilder;
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
                    log::debug!("[Component-{}] readiness check passed for target: {}", self.identifier, target);
                }
                Err(msg) => {
                    panic!("[Component-{}] readiness check failed for target {}: {}", self.identifier, target, msg);
                }
            }
        }
        log::debug!("[Component-{}] all readiness checks passed.", self.identifier);
    }

    fn log_line(identifier: &str, line: &str) {
        if line.contains(" ERROR ") {
            log::error!("[{}] {}", identifier, line);
        } else if line.contains(" WARN ") {
            log::warn!("[{}] {}", identifier, line);
        } else if line.contains(" DEBUG ") {
            log::debug!("[{}] {}", identifier, line);
        } else if line.contains(" TRACE ") {
            log::trace!("[{}] {}", identifier, line);
        } else {
            log::info!("[{}] {}", identifier, line);
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
        let executable_path = self.executable_path.as_ref()
            .ok_or_else(|| "executable_path not configured".to_string())?;

        log::info!("[Component-{}] spawning process: {:?}", self.identifier, executable_path);

        let mut cmd = Command::new(executable_path);
        
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        for (_key, value) in &self.runtime_args {
            cmd.arg(value);
        }

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn process: {}", e))?;

        let pid = child.id();
        log::info!("[Component-{}] process spawned (pid: {})", self.identifier, pid);

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

        log::info!("[Component-{}] starting.", self.identifier);

        if self.executable_path.is_some() {
            if let Err(e) = self.spawn_process() {
                panic!("[Component-{}] spawn failed: {}", self.identifier, e);
            }
        }

        self.wait_until_ready().await;

        log::info!("[Component-{}] started.", self.identifier);
    }

    async fn stop(&mut self) {
        if self.stopped {
            return;
        }

        log::info!("[Component-{}] stopping.", self.identifier);

        if let Some(mut child) = self.process_handle.take() {
            log::info!("[Component-{}] killing process (pid: {})", self.identifier, child.id());
            let _ = child.kill();
            let _ = child.wait();
        }

        log::info!("[Component-{}] stopped.", self.identifier);

        for child in self.children.iter_mut().flatten().rev() {
            child.stop().await;
        }

        self.stopped = true;
    }

    fn add_child(&mut self, child: Box<dyn RunnableComponent>) {
        self.children.get_or_insert_with(Vec::new).push(child);
    }
}