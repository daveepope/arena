use arena::component::RunnableComponent;
use arena_executable_component::executable_component::ExecutableComponent;
use arena_executable_component::BuildTool;

#[test]
fn build_missing_source_path_returns_fault() {
    let fault = ExecutableComponent::builder("exec-missing-source")
        .with_source_path("arena-nonexistent-source-tree-4c1d9a")
        .with_build_tool(BuildTool::Cargo)
        .build()
        .err()
        .expect("missing source path must fault");
    assert!(fault.id.contains("exec-missing-source"));
    assert!(fault.message.contains("could not find source path"));
}

#[test]
fn build_missing_custom_build_command_returns_fault() {
    let fault = ExecutableComponent::builder("exec-missing-build-command")
        .with_source_path(".")
        .with_build_tool(BuildTool::Custom {
            command: "arena-nonexistent-build-command-4c1d9a".to_string(),
            args: Vec::new(),
        })
        .build()
        .err()
        .expect("missing build command must fault");
    assert!(fault.message.contains("failed to run custom build command"));
}

#[cfg(unix)]
#[test]
fn build_failing_custom_build_command_returns_fault() {
    let fault = ExecutableComponent::builder("exec-failing-build-command")
        .with_source_path(".")
        .with_build_tool(BuildTool::Custom {
            command: "false".to_string(),
            args: Vec::new(),
        })
        .build()
        .err()
        .expect("failing build command must fault");
    assert!(fault.message.contains("build failed"));
}

#[test]
fn build_executable_path_only_returns_component() {
    let component = ExecutableComponent::builder("exec-plain")
        .with_executable_path("some/relative/binary")
        .build()
        .expect("build executable component");
    assert!(component.identifier().contains("exec-plain"));
}

#[test]
fn build_no_executable_path_returns_component() {
    let component = ExecutableComponent::builder("exec-no-path")
        .build()
        .expect("build executable component");
    assert!(component.identifier().contains("exec-no-path"));
}

#[test]
fn build_absolute_executable_path_returns_component() {
    let absolute = std::env::temp_dir().join("arena-exec-absolute-probe");
    let component = ExecutableComponent::builder("exec-absolute")
        .with_executable_path(absolute)
        .build()
        .expect("build executable component");
    assert!(component.identifier().contains("exec-absolute"));
}

#[cfg(unix)]
#[test]
fn build_succeeding_custom_build_command_returns_component() {
    let component = ExecutableComponent::builder("exec-build-ok")
        .with_source_path(".")
        .with_build_tool(BuildTool::Custom {
            command: "true".to_string(),
            args: Vec::new(),
        })
        .build()
        .expect("build executable component");
    assert!(component.identifier().contains("exec-build-ok"));
}

#[test]
fn build_make_in_empty_source_dir_returns_fault() {
    let source = std::env::temp_dir().join(format!(
        "arena-exec-make-probe-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&source).expect("create probe source dir");
    let fault = ExecutableComponent::builder("exec-make-empty")
        .with_source_path(&source)
        .with_build_tool(BuildTool::Make)
        .build()
        .err()
        .expect("make in an empty source dir must fault");
    let _ = std::fs::remove_dir_all(&source);
    assert!(fault.message.contains("build failed") || fault.message.contains("failed to run make"));
}

#[test]
fn attach_cpu_profiler_supported_build_tool_returns_ok() {
    for build_tool in [
        BuildTool::Cargo,
        BuildTool::Maven,
        BuildTool::Gradle,
        BuildTool::Python,
        BuildTool::Dotnet,
    ] {
        let mut component = ExecutableComponent::builder("exec-cpu-profile-supported")
            .with_build_tool(build_tool)
            .with_executable_path("/bin/true")
            .build()
            .expect("build executable component");

        assert!(component.attach_cpu_profiler(None).is_ok());
    }
}

#[test]
fn attach_cpu_profiler_unsupported_build_tool_returns_fault_naming_the_tool() {
    for (build_tool, expected) in [
        (BuildTool::Make, "BuildTool::Make"),
        (BuildTool::CMake, "BuildTool::CMake"),
        (
            BuildTool::Custom {
                command: "make-it-so".to_string(),
                args: Vec::new(),
            },
            "BuildTool::Custom(\"make-it-so\")",
        ),
    ] {
        let mut component = ExecutableComponent::builder("exec-cpu-profile-unsupported")
            .with_build_tool(build_tool)
            .with_executable_path("/bin/true")
            .build()
            .expect("build executable component");

        let fault = component
            .attach_cpu_profiler(None)
            .err()
            .expect("unsupported build tool must fault");

        assert!(fault.id.contains("exec-cpu-profile-unsupported"));
        assert!(fault.message.contains("cpu profiling is not supported for"));
        assert!(fault.message.contains(expected));
    }
}

#[test]
fn attach_cpu_profiler_without_build_tool_returns_fault() {
    let mut component = ExecutableComponent::builder("exec-cpu-profile-no-build-tool")
        .with_executable_path("/bin/true")
        .build()
        .expect("build executable component");

    let fault = component
        .attach_cpu_profiler(None)
        .err()
        .expect("missing build tool must fault");

    assert!(fault
        .message
        .contains("requires a build_tool of Cargo, Maven, Gradle, Python, or Dotnet"));
}
