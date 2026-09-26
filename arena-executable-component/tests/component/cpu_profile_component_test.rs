use arena::component::RunnableComponent;
use arena_executable_component::builder::BuildTool;
use arena_executable_component::executable_component::ExecutableComponent;
use std::time::Duration;

const BUSY_LOOP_JAVA: &str = "import java.nio.file.Files;\nimport java.nio.file.Path;\npublic class BusyLoop {\n    public static void main(String[] args) throws Exception {\n        Files.write(Path.of(args[0]), new byte[] {1});\n        long i = 0;\n        while (true) {\n            i++;\n        }\n    }\n}\n";

const JVM_START_BUDGET: Duration = Duration::from_secs(20);
const SAMPLING_WINDOW: Duration = Duration::from_millis(500);

async fn await_jvm_running(marker_path: &std::path::Path, component: &mut ExecutableComponent) {
    let deadline = std::time::Instant::now() + JVM_START_BUDGET;
    while std::time::Instant::now() < deadline {
        if marker_path.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    component.force_stop().await;
    panic!("jvm did not reach main() within {JVM_START_BUDGET:?}");
}

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
        .build()
        .expect("build executable component");

    component.start().await.expect("start component");
    tokio::time::sleep(Duration::from_millis(500)).await;
    component.stop().await.expect("stop component");

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
}

#[tokio::test]
async fn stop_pyspy_wrapped_profile_with_hotspots_renders_hotspots_table() {
    let output_path = std::env::temp_dir().join(format!(
        "arena-executable-component-pyspy-hotspots-test-{}.html",
        std::process::id()
    ));

    let mut component = ExecutableComponent::builder("cpu-profile-pyspy-hotspots-test")
        .with_build_tool(BuildTool::Python)
        .with_executable_path("/usr/bin/python3")
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", "while True:\n    pass\n")
        .with_cpu_profile(&output_path)
        .with_hotspots()
        .build()
        .expect("build executable component");

    component.start().await.expect("start component");
    tokio::time::sleep(Duration::from_millis(500)).await;
    component.stop().await.expect("stop component");

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    assert!(report.contains("arena-profile-hotspots"));
    assert!(report.contains("severity-badge"));
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

    let marker_path = std::env::temp_dir().join(format!(
        "arena-executable-component-asprof-running-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&marker_path);

    let mut component = ExecutableComponent::builder("cpu-profile-asprof-test")
        .with_build_tool(BuildTool::Maven)
        .with_executable_path("/usr/bin/java")
        .with_runtime_arg("fixture", fixture_path.to_string_lossy().into_owned())
        .with_runtime_arg("marker", marker_path.to_string_lossy().into_owned())
        .with_cpu_profile(&output_path)
        .build()
        .expect("build executable component");

    component.start().await.expect("start component");
    await_jvm_running(&marker_path, &mut component).await;
    tokio::time::sleep(SAMPLING_WINDOW).await;
    component.stop().await.expect("stop component");

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_file(&fixture_path);
    let _ = std::fs::remove_file(&marker_path);
}
