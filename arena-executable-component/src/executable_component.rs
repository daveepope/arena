use async_trait::async_trait;
use arena::component::RunnableComponent;
use crate::builder::ExecutableComponentBuilder;
use std::path::PathBuf;
use std::process::{Child, Command};

pub struct ExecutableComponent {
    pub(crate) endpoint: String,
    pub(crate) children: Option<Vec<Box<dyn RunnableComponent>>>,
    pub(crate) source_path: Option<PathBuf>,
    pub(crate) executable_path: Option<PathBuf>,
    pub(crate) env_vars: Vec<(String, String)>,
    pub(crate) runtime_args: Vec<(String, String)>,
    pub(crate) process_handle: Option<Child>,
    pub(crate) stopped: bool,
}

impl ExecutableComponent {
    pub fn new(endpoint: String) -> Self {
        ExecutableComponent {
            endpoint,
            children: None,
            source_path: None,
            executable_path: None,
            env_vars: Vec::new(),
            runtime_args: Vec::new(),
            process_handle: None,
            stopped: false,
        }
    }

    pub fn builder(identifier: impl Into<String>) -> ExecutableComponentBuilder {
        ExecutableComponentBuilder::new(identifier)
    }


    fn spawn_process(&mut self) -> Result<(), String> {
        let executable_path = self.executable_path.as_ref()
            .ok_or_else(|| "executable_path not configured".to_string())?;

        log::info!("[Component-{}] spawning process: {:?}", self.endpoint, executable_path);

        let mut cmd = Command::new(executable_path);
        
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        for (_key, value) in &self.runtime_args {
            cmd.arg(value);
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn process: {}", e))?;

        self.process_handle = Some(child);
        log::info!("[Component-{}] process spawned (pid: {:?})", self.endpoint, self.process_handle.as_ref().map(|c| c.id()));
        
        Ok(())
    }
    
}

#[async_trait]
impl RunnableComponent for ExecutableComponent {
    async fn start(&mut self) {
        for child in self.children.iter_mut().flatten() {
            child.start().await;
        }

        log::info!("[Component-{}] starting.", self.endpoint);

        if self.executable_path.is_some() {
            if let Err(e) = self.spawn_process() {
                log::error!("[Component-{}] spawn failed: {}", self.endpoint, e);
                return;
            }
        }

        log::info!("[Component-{}] started.", self.endpoint);
    }

    async fn stop(&mut self) {
        if self.stopped {
            return;
        }

        log::info!("[Component-{}] stopping.", self.endpoint);

        if let Some(mut child) = self.process_handle.take() {
            log::info!("[Component-{}] killing process (pid: {})", self.endpoint, child.id());
            let _ = child.kill();
            let _ = child.wait();
        }

        log::info!("[Component-{}] stopped.", self.endpoint);

        for child in self.children.iter_mut().flatten().rev() {
            child.stop().await;
        }

        self.stopped = true;
    }

    fn add_child(&mut self, child: Box<dyn RunnableComponent>) {
        self.children.get_or_insert_with(Vec::new).push(child);
    }
}