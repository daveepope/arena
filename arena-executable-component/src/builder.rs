use crate::executable_component::ExecutableComponent;
use crate::platform::resolve_executable_extension;
use arena::healthcheck::ReadinessCheck;
use arena::Component;
use arena::Fault;
use std::path::PathBuf;

pub enum BuildTool {
    Cargo,
    Maven,
    Gradle,
    Dotnet,
    Make,
    CMake,
    Custom { command: String, args: Vec<String> },
}

pub struct ExecutableComponentBuilder {
    identifier: String,
    children: Option<Vec<Component>>,
    source_path: Option<PathBuf>,
    build_tool: Option<BuildTool>,
    executable_path: Option<PathBuf>,
    env_vars: Vec<(String, String)>,
    runtime_args: Vec<(String, String)>,
    readiness_checks: Vec<(Box<dyn ReadinessCheck>, String, u64)>,
}

const DEFAULT_READINESS_TIMEOUT_MS: u64 = 10_000;

impl ExecutableComponentBuilder {
    pub(crate) fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: arena_container::identifier::build(
                "arena-executable-component",
                &identifier.into(),
            ),
            children: None,
            source_path: None,
            build_tool: None,
            executable_path: None,
            env_vars: Vec::new(),
            runtime_args: Vec::new(),
            readiness_checks: Vec::new(),
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

    pub fn with_build_tool(mut self, build_tool: BuildTool) -> Self {
        self.build_tool = Some(build_tool);
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

    pub fn with_readiness_check<R>(self, check: R, target: impl Into<String>) -> Self
    where
        R: ReadinessCheck + 'static,
    {
        self.with_readiness_check_timeout(check, target, DEFAULT_READINESS_TIMEOUT_MS)
    }

    pub fn with_readiness_check_timeout<R>(
        mut self,
        check: R,
        target: impl Into<String>,
        timeout_ms: u64,
    ) -> Self
    where
        R: ReadinessCheck + 'static,
    {
        self.readiness_checks
            .push((Box::new(check), target.into(), timeout_ms));
        self
    }

    pub fn build(self) -> Result<ExecutableComponent, Fault> {
        if let (Some(ref source_path), Some(ref build_tool)) = (&self.source_path, &self.build_tool)
        {
            tracing::debug!(
                component = %self.identifier,
                source_path = ?source_path,
                phase = "executable_build_begin",
                "building executable from source tree",
            );

            let source_dir = if source_path.is_absolute() {
                source_path.clone()
            } else {
                // Walk up to find a directory where source_path exists
                let current_dir = current_dir_fault(&self.identifier)?;

                current_dir
                    .ancestors()
                    .find_map(|ancestor| {
                        let candidate = ancestor.join(source_path);
                        if candidate.exists() {
                            Some(candidate)
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| {
                        Fault::component(
                            self.identifier.clone(),
                            format!(
                                "could not find source path '{}' from current directory or any parent",
                                source_path.display()
                            ),
                        )
                    })?
            };

            let output = Self::execute_build(&self.identifier, build_tool, &source_dir)?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Fault::component(
                    self.identifier.clone(),
                    format!("build failed: {}", stderr),
                ));
            }

            tracing::debug!(
                component = %self.identifier,
                phase = "executable_build_done",
                "build finished",
            );
        }

        let executable_path = match self.executable_path {
            Some(path) => {
                let resolved = if path.is_absolute() {
                    path
                } else {
                    let current_dir = current_dir_fault(&self.identifier)?;

                    current_dir
                        .ancestors()
                        .find_map(|ancestor| {
                            let candidate = ancestor.join(&path);
                            if candidate.exists() {
                                Some(candidate)
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| current_dir.join(&path))
                };
                Some(resolve_executable_extension(resolved))
            }
            None => None,
        };

        let mut component = ExecutableComponent::new(self.identifier);
        component.children = self.children;
        component.executable_path = executable_path;
        component.env_vars = self.env_vars;
        component.runtime_args = self.runtime_args;
        component.readiness_checks = self.readiness_checks;
        Ok(component)
    }

    fn execute_build(
        identifier: &str,
        build_tool: &BuildTool,
        source_dir: &PathBuf,
    ) -> Result<std::process::Output, Fault> {
        let (command, args): (&str, &[&str]) = match build_tool {
            BuildTool::Cargo => ("cargo", &["build", "--release"]),
            BuildTool::Maven => ("mvn", &["clean", "package"]),
            BuildTool::Gradle => ("gradle", &["build"]),
            BuildTool::Dotnet => ("dotnet", &["build", "--configuration", "Release"]),
            BuildTool::Make => ("make", &[]),
            BuildTool::CMake => ("cmake", &["--build", ".", "--config", "Release"]),
            BuildTool::Custom { command, args } => {
                return std::process::Command::new(command)
                    .args(args)
                    .current_dir(source_dir)
                    .output()
                    .map_err(|e| {
                        Fault::component(
                            identifier,
                            format!("failed to run custom build command {command}: {e}"),
                        )
                    });
            }
        };
        std::process::Command::new(command)
            .args(args)
            .current_dir(source_dir)
            .output()
            .map_err(|e| Fault::component(identifier, format!("failed to run {command}: {e}")))
    }
}

fn current_dir_fault(identifier: &str) -> Result<PathBuf, Fault> {
    std::env::current_dir().map_err(|e| {
        Fault::component(identifier, format!("failed to read current directory: {e}"))
    })
}
