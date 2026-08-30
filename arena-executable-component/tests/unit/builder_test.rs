use arena_executable_component::builder::{
    BuildTool, DOTNET_PERF_MAP_ENV_VAR, DOTNET_PERF_MAP_ENV_VALUE,
};
use arena_executable_component::executable_component::ExecutableComponent;
use arena_profiler::CpuProfilerBackend;
use std::path::PathBuf;

const PROFILE_OUTPUT: &str = "/tmp/arena-executable-component-builder-test.html";

fn build_with_cpu_profile(build_tool: BuildTool) -> ExecutableComponent {
    ExecutableComponent::builder("builder-test")
        .with_build_tool(build_tool)
        .with_executable_path("/bin/true")
        .with_cpu_profile(PROFILE_OUTPUT)
        .build()
}

#[test]
fn build_cargo_with_cpu_profile_selects_perf_backend() {
    let component = build_with_cpu_profile(BuildTool::Cargo);

    let config = component.cpu_profile.expect("cpu profile configured");
    assert_eq!(config.backend, CpuProfilerBackend::Perf);
    assert!(!config.auto_open);
    assert!(!config.include_hotspots);
}

#[test]
fn build_dotnet_with_cpu_profile_selects_perf_backend() {
    let component = build_with_cpu_profile(BuildTool::Dotnet);

    let config = component.cpu_profile.expect("cpu profile configured");
    assert_eq!(config.backend, CpuProfilerBackend::Perf);
}

#[test]
fn build_dotnet_with_cpu_profile_enables_perf_map_env_var() {
    let component = build_with_cpu_profile(BuildTool::Dotnet);

    let config = component.cpu_profile.expect("cpu profile configured");
    assert_eq!(
        config.env_vars,
        vec![(
            DOTNET_PERF_MAP_ENV_VAR.to_string(),
            DOTNET_PERF_MAP_ENV_VALUE.to_string()
        )]
    );
}

#[test]
fn build_dotnet_without_cpu_profile_omits_perf_map_env_var() {
    let component = ExecutableComponent::builder("builder-test")
        .with_build_tool(BuildTool::Dotnet)
        .with_executable_path("/bin/true")
        .build();

    assert!(component.cpu_profile.is_none());
    assert!(component.env_vars.is_empty());
}

#[test]
fn build_maven_or_gradle_with_cpu_profile_selects_async_profiler_backend() {
    for build_tool in [BuildTool::Maven, BuildTool::Gradle] {
        let component = build_with_cpu_profile(build_tool);

        let config = component.cpu_profile.expect("cpu profile configured");
        assert_eq!(config.backend, CpuProfilerBackend::AsyncProfiler);
    }
}

#[test]
fn build_python_with_cpu_profile_selects_pyspy_backend() {
    let component = build_with_cpu_profile(BuildTool::Python);

    let config = component.cpu_profile.expect("cpu profile configured");
    assert_eq!(config.backend, CpuProfilerBackend::PySpy);
    assert!(config.env_vars.is_empty());
}

#[test]
fn build_non_dotnet_with_cpu_profile_omits_perf_map_env_var() {
    for build_tool in [BuildTool::Cargo, BuildTool::Maven, BuildTool::Python] {
        let component = build_with_cpu_profile(build_tool);

        let config = component.cpu_profile.expect("cpu profile configured");
        assert!(config.env_vars.is_empty());
    }
}

#[test]
fn build_with_cpu_profile_auto_open_sets_auto_open() {
    let component = ExecutableComponent::builder("builder-test")
        .with_build_tool(BuildTool::Python)
        .with_executable_path("/bin/true")
        .with_cpu_profile(PROFILE_OUTPUT)
        .with_cpu_profile_auto_open()
        .build();

    let config = component.cpu_profile.expect("cpu profile configured");
    assert!(config.auto_open);
}

#[test]
fn build_with_hotspots_sets_include_hotspots() {
    let component = ExecutableComponent::builder("builder-test")
        .with_build_tool(BuildTool::Cargo)
        .with_executable_path("/bin/true")
        .with_cpu_profile(PROFILE_OUTPUT)
        .with_hotspots()
        .build();

    let config = component.cpu_profile.expect("cpu profile configured");
    assert!(config.include_hotspots);
}

#[test]
fn build_with_cpu_profile_records_output_path() {
    let component = build_with_cpu_profile(BuildTool::Cargo);

    let config = component.cpu_profile.expect("cpu profile configured");
    assert_eq!(config.output_path, PathBuf::from(PROFILE_OUTPUT));
}

#[test]
#[should_panic(expected = "is not supported for BuildTool::Make")]
fn build_make_with_cpu_profile_panics() {
    build_with_cpu_profile(BuildTool::Make);
}

#[test]
#[should_panic(expected = "is not supported for BuildTool::CMake")]
fn build_cmake_with_cpu_profile_panics() {
    build_with_cpu_profile(BuildTool::CMake);
}

#[test]
#[should_panic(expected = "is not supported for BuildTool::Custom")]
fn build_custom_with_cpu_profile_panics() {
    build_with_cpu_profile(BuildTool::Custom {
        command: "build-it".to_string(),
        args: vec![],
    });
}

#[test]
#[should_panic(expected = "requires a build_tool of Cargo, Dotnet, Maven, Gradle, or Python")]
fn build_no_build_tool_with_cpu_profile_panics() {
    ExecutableComponent::builder("builder-test")
        .with_executable_path("/bin/true")
        .with_cpu_profile(PROFILE_OUTPUT)
        .build();
}

#[test]
fn build_absolute_executable_path_kept_as_is() {
    let component = ExecutableComponent::builder("builder-test")
        .with_executable_path("/bin/true")
        .build();

    assert_eq!(component.executable_path, Some(PathBuf::from("/bin/true")));
}

#[test]
fn build_relative_executable_path_not_found_falls_back_to_current_dir() {
    let relative = PathBuf::from("arena-executable-component-nonexistent-binary");

    let component = ExecutableComponent::builder("builder-test")
        .with_executable_path(relative.clone())
        .build();

    let expected = std::env::current_dir().unwrap().join(&relative);
    assert_eq!(component.executable_path, Some(expected));
}

#[test]
fn build_no_cpu_profile_leaves_profile_unset() {
    let component = ExecutableComponent::builder("builder-test")
        .with_build_tool(BuildTool::Cargo)
        .with_executable_path("/bin/true")
        .build();

    assert!(component.cpu_profile.is_none());
}

fn temp_source_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "arena-executable-component-source-{name}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create source dir");
    dir
}

fn custom_build_tool(command: &str) -> BuildTool {
    BuildTool::Custom { command: command.to_string(), args: vec![] }
}

#[test]
fn build_absolute_source_path_runs_custom_build_tool() {
    let source_dir = temp_source_dir("absolute");

    let component = ExecutableComponent::builder("builder-test")
        .with_source_path(&source_dir)
        .with_build_tool(custom_build_tool("true"))
        .with_executable_path("/bin/true")
        .build();

    assert_eq!(component.executable_path, Some(PathBuf::from("/bin/true")));
    let _ = std::fs::remove_dir_all(&source_dir);
}

#[test]
fn build_relative_source_path_resolves_against_ancestors() {
    let component = ExecutableComponent::builder("builder-test")
        .with_source_path(".")
        .with_build_tool(custom_build_tool("true"))
        .with_executable_path("/bin/true")
        .build();

    assert_eq!(component.executable_path, Some(PathBuf::from("/bin/true")));
}

#[test]
#[should_panic(expected = "build failed")]
fn build_failing_build_tool_panics() {
    let source_dir = temp_source_dir("failing");

    ExecutableComponent::builder("builder-test")
        .with_source_path(&source_dir)
        .with_build_tool(custom_build_tool("false"))
        .build();
}

#[test]
fn build_python_with_source_path_skips_the_build_step() {
    let source_dir = temp_source_dir("python");

    let component = ExecutableComponent::builder("builder-test")
        .with_source_path(&source_dir)
        .with_build_tool(BuildTool::Python)
        .with_executable_path("/bin/true")
        .build();

    assert_eq!(component.executable_path, Some(PathBuf::from("/bin/true")));
    let _ = std::fs::remove_dir_all(&source_dir);
}

#[test]
#[should_panic(expected = "could not find source path")]
fn build_missing_relative_source_path_panics() {
    ExecutableComponent::builder("builder-test")
        .with_source_path("arena-executable-component-nonexistent-source-dir")
        .with_build_tool(custom_build_tool("true"))
        .build();
}

#[test]
fn build_with_child_components_records_children() {
    let child = ExecutableComponent::builder("child").with_executable_path("/bin/true").build();

    let component = ExecutableComponent::builder("builder-test")
        .with_child_components(vec![Box::new(child)])
        .with_executable_path("/bin/true")
        .build();

    assert_eq!(component.executable_path, Some(PathBuf::from("/bin/true")));
}

#[test]
fn build_env_vars_and_runtime_args_are_recorded() {
    let component = ExecutableComponent::builder("builder-test")
        .with_executable_path("/bin/true")
        .with_env_var("KEY", "value")
        .with_runtime_arg("flag", "-c")
        .build();

    assert_eq!(component.env_vars, vec![("KEY".to_string(), "value".to_string())]);
    assert_eq!(component.runtime_args, vec![("flag".to_string(), "-c".to_string())]);
}
