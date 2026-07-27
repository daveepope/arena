use arena::component::RunnableComponent;
use arena_executable_component::builder::BuildTool;
use arena_executable_component::executable_component::ExecutableComponent;
use std::time::Duration;

const BUSY_LOOP_JAVA: &str = "public class BusyLoop {\n    public static void main(String[] args) {\n        long i = 0;\n        while (true) {\n            i++;\n        }\n    }\n}\n";

#[tokio::test]
async fn stop_pyspy_wrapped_profile_configured_renders_html_report() {
    let output_path = std::env::temp_dir().join(format!(
        "arena-executable-component-pyspy-cpu-profile-test-{}.html",
        std::process::id()
    ));

    let mut component = ExecutableComponent::builder("cpu-profile-pyspy-test")
        .with_build_tool(BuildTool::Python)
        .with_executable_path("/usr/bin/python3")
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", "while True:\n    pass\n")
        .with_cpu_profile(&output_path)
        .build();

    component.start().await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    component.stop().await;

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn stop_async_profiler_arg_augmented_profile_configured_renders_html_report() {
    let fixture_path = std::env::temp_dir().join(format!(
        "arena-executable-component-asprof-fixture-{}.java",
        std::process::id()
    ));
    std::fs::write(&fixture_path, BUSY_LOOP_JAVA).expect("write java fixture");

    let output_path = std::env::temp_dir().join(format!(
        "arena-executable-component-asprof-cpu-profile-test-{}.html",
        std::process::id()
    ));

    let mut component = ExecutableComponent::builder("cpu-profile-asprof-test")
        .with_build_tool(BuildTool::Maven)
        .with_executable_path("/usr/bin/java")
        .with_runtime_arg("fixture", fixture_path.to_string_lossy().into_owned())
        .with_cpu_profile(&output_path)
        .build();

    component.start().await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    component.stop().await;

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_file(&fixture_path);
}
