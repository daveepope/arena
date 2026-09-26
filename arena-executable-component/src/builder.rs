use crate::executable_component::{CpuProfileConfig, ExecutableComponent};
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
    Python,
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
    cpu_profile_output: Option<PathBuf>,
    cpu_profile_auto_open: bool,
    cpu_profile_hotspots: bool,
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
            cpu_profile_output: None,
            cpu_profile_auto_open: false,
            cpu_profile_hotspots: false,
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

    pub fn with_cpu_profile(mut self, output_path: impl Into<PathBuf>) -> Self {
        self.cpu_profile_output = Some(output_path.into());
        self
    }

    pub fn with_cpu_profile_auto_open(mut self) -> Self {
        self.cpu_profile_auto_open = true;
        self
    }

    pub fn with_hotspots(mut self) -> Self {
        self.cpu_profile_hotspots = true;
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

            if !matches!(build_tool, BuildTool::Python) {
                let output = Self::execute_build(&self.identifier, build_tool, &source_dir)?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(Fault::component(
                        self.identifier.clone(),
                        format!("build failed: {}", stderr),
                    ));
                }
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

        let cpu_profile = match self.cpu_profile_output {
            Some(output_path) => Some(CpuProfileConfig {
                backend: cpu_profiler_backend(&self.identifier, self.build_tool.as_ref())?,
                output_path,
                auto_open: self.cpu_profile_auto_open,
                include_hotspots: self.cpu_profile_hotspots,
            }),
            None => None,
        };

        let mut component = ExecutableComponent::new(self.identifier);
        component.children = self.children;
        component.executable_path = executable_path;
        component.env_vars = self.env_vars;
        component.runtime_args = self.runtime_args;
        component.readiness_checks = self.readiness_checks;
        component.cpu_profile = cpu_profile;
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
            BuildTool::Python => {
                return Err(Fault::component(
                    identifier,
                    "python executable components are launched directly, not built from source",
                ));
            }
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

fn cpu_profiler_backend(
    identifier: &str,
    build_tool: Option<&BuildTool>,
) -> Result<arena_profile::CpuProfilerBackend, Fault> {
    match build_tool {
        Some(BuildTool::Cargo) => Ok(arena_profile::CpuProfilerBackend::Perf),
        Some(BuildTool::Maven) | Some(BuildTool::Gradle) => {
            Ok(arena_profile::CpuProfilerBackend::AsyncProfiler)
        }
        Some(BuildTool::Python) => Ok(arena_profile::CpuProfilerBackend::PySpy),
        Some(BuildTool::Dotnet) => Err(cpu_profile_unsupported(identifier, "BuildTool::Dotnet")),
        Some(BuildTool::Make) => Err(cpu_profile_unsupported(identifier, "BuildTool::Make")),
        Some(BuildTool::CMake) => Err(cpu_profile_unsupported(identifier, "BuildTool::CMake")),
        Some(BuildTool::Custom { command, .. }) => Err(cpu_profile_unsupported(
            identifier,
            &format!("BuildTool::Custom(\"{command}\")"),
        )),
        None => Err(Fault::component(
            identifier,
            ".with_cpu_profile() requires a build_tool of Cargo, Maven, Gradle, or Python",
        )),
    }
}

fn cpu_profile_unsupported(identifier: &str, build_tool: &str) -> Fault {
    Fault::component(
        identifier,
        format!(".with_cpu_profile() is not supported for {build_tool}"),
    )
}
