use arena::component::RunnableComponent;
use arena_executable_component::builder::BuildTool;
use arena_executable_component::executable_component::ExecutableComponent;
use std::path::PathBuf;
use std::time::Duration;

const BUSY_LOOP_JAVA: &str = "public class BusyLoop {\n    public static void main(String[] args) {\n        long i = 0;\n        while (true) {\n            i++;\n        }\n    }\n}\n";
const BUSY_LOOP_PYTHON: &str = "while True:\n    pass\n";

fn temp_path(name: &str, ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "arena-executable-component-{name}-{}.{ext}",
        std::process::id()
    ))
}

fn runtime_binary(env_var: &str, relative_binary: &str) -> PathBuf {
    let runtime = std::env::var(env_var).unwrap_or_else(|_| panic!("{env_var} is set by BUILD"));
    let candidate = PathBuf::from(&runtime);
    if candidate.is_file() {
        return candidate;
    }
    let with_binary = candidate.join(relative_binary);
    if with_binary.exists() {
        return with_binary;
    }
    std::env::current_dir()
        .expect("read current directory")
        .join(&runtime)
        .join(relative_binary)
}

fn python_binary() -> PathBuf {
    runtime_binary("ARENA_PYTHON_RUNTIME", "bin/python3")
}

fn java_binary() -> PathBuf {
    runtime_binary("ARENA_JAVA_RUNTIME", "bin/java")
}

async fn profile_briefly(component: &mut ExecutableComponent) {
    component.start().await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    component.stop().await;
}

#[tokio::test]
async fn stop_pyspy_wrapped_profile_configured_renders_html_report() {
    let output_path = temp_path("pyspy-cpu-profile", "html");

    let mut component = ExecutableComponent::builder("cpu-profile-pyspy-test")
        .with_build_tool(BuildTool::Python)
        .with_executable_path(python_binary())
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", BUSY_LOOP_PYTHON)
        .with_cpu_profile(&output_path)
        .build();

    profile_briefly(&mut component).await;

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn stop_pyspy_wrapped_profile_with_hotspots_renders_hotspots_table() {
    let output_path = temp_path("pyspy-hotspots", "html");

    let mut component = ExecutableComponent::builder("cpu-profile-pyspy-hotspots-test")
        .with_build_tool(BuildTool::Python)
        .with_executable_path(python_binary())
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", BUSY_LOOP_PYTHON)
        .with_cpu_profile(&output_path)
        .with_hotspots()
        .build();

    profile_briefly(&mut component).await;

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    assert!(report.contains("arena-profiler-hotspots"));
    assert!(report.contains("severity-badge"));
    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn stop_async_profiler_arg_augmented_profile_configured_renders_html_report() {
    let fixture_path = temp_path("asprof-fixture", "java");
    std::fs::write(&fixture_path, BUSY_LOOP_JAVA).expect("write java fixture");
    let output_path = temp_path("asprof-cpu-profile", "html");

    let mut component = ExecutableComponent::builder("cpu-profile-asprof-test")
        .with_build_tool(BuildTool::Maven)
        .with_executable_path(java_binary())
        .with_runtime_arg("fixture", fixture_path.to_string_lossy().into_owned())
        .with_cpu_profile(&output_path)
        .build();

    profile_briefly(&mut component).await;

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    assert!(report.contains("BusyLoop"), "expected resolved managed frames in the report");
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_file(&fixture_path);
}
