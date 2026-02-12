use arena::Component;
use crate::executable_component::ExecutableComponent;
use std::path::PathBuf;

pub struct ExecutableComponentBuilder {
    endpoint: String,
    children: Option<Vec<Component>>,
    source_path: Option<PathBuf>,
    executable_path: Option<PathBuf>,
    env_vars: Vec<(String, String)>,
    runtime_args: Vec<(String, String)>,
}

impl ExecutableComponentBuilder {
    pub(crate) fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            children: None,
            source_path: None,
            executable_path: None,
            env_vars: Vec::new(),
            runtime_args: Vec::new(),
        }
    }

    pub fn with_child_components(mut self, children: Vec<Component>) -> Self {
        self.children = Some(children);
        self
    }

    pub fn with_source_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    pub fn with_executable_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.executable_path = Some(path.into());
        self
    }

    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }

    pub fn with_runtime_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.runtime_args.push((key.into(), value.into()));
        self
    }

    pub fn build(self) -> ExecutableComponent {
        if let Some(ref source_path) = self.source_path {
            log::info!("[Component-{}] building executable from {:?}", self.endpoint, source_path);
            
            let output = std::process::Command::new("cargo")
                .args(&["build", "--release", "--manifest-path"])
                .arg(source_path.join("Cargo.toml"))
                .output()
                .expect("failed to run cargo build");

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                panic!("cargo build failed: {}", stderr);
            }

            log::info!("[Component-{}] build complete", self.endpoint);
        }

        ExecutableComponent {
            endpoint: self.endpoint,
            children: self.children,
            source_path: self.source_path,
            executable_path: self.executable_path,
            env_vars: self.env_vars,
            runtime_args: self.runtime_args,
            process_handle: None,
            stopped: false,
        }
    }
}