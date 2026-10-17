#[path = "../support/recorded_profiles.rs"]
mod recorded_profiles;

use arena::component::RunnableComponent;
use arena::lifecycle::RunnableState;
use arena_executable_component::builder::BuildTool;
use arena_executable_component::executable_component::ExecutableComponent;
use recorded_profiles::start_recording;
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
async fn stop_pyspy_wrapped_profile_attached_reports_folded_stacks() {
    let recording = start_recording();

    let mut component = ExecutableComponent::builder("cpu-profile-pyspy-test")
        .with_build_tool(BuildTool::Python)
        .with_executable_path("/usr/bin/python3")
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", "while True:\n    pass\n")
        .build()
        .expect("build executable component");
    component
        .attach_cpu_profiler()
        .expect("attach cpu profiler");
    let component_id = component.identifier().to_string();

    component.start().await.expect("start component");
    tokio::time::sleep(Duration::from_millis(500)).await;
    component.stop().await.expect("stop component");

    let folded = recording
        .recorded
        .folded_for(&component_id)
        .expect("folded stacks were not reported");
    assert!(!folded.trim().is_empty(), "folded stacks were empty");
}

#[tokio::test]
async fn stop_pyspy_wrapped_profile_attached_reports_the_full_transition_sequence() {
    let recording = start_recording();

    let mut component = ExecutableComponent::builder("cpu-profile-pyspy-transitions-test")
        .with_build_tool(BuildTool::Python)
        .with_executable_path("/usr/bin/python3")
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", "while True:\n    pass\n")
        .build()
        .expect("build executable component");
    component
        .attach_cpu_profiler()
        .expect("attach cpu profiler");
    let component_id = component.identifier().to_string();

    component.start().await.expect("start component");
    tokio::time::sleep(Duration::from_millis(500)).await;
    component.stop().await.expect("stop component");

    assert_eq!(
        recording.recorded.states_for(&component_id),
        vec![
            RunnableState::Starting,
            RunnableState::Started,
            RunnableState::Stopping,
            RunnableState::Stopped,
        ]
    );
}

#[tokio::test]
async fn stop_async_profiler_arg_augmented_profile_attached_reports_folded_stacks() {
    let fixture_path = std::env::temp_dir().join(format!(
        "arena-executable-component-asprof-fixture-{}.java",
        std::process::id()
    ));
    std::fs::write(&fixture_path, BUSY_LOOP_JAVA).expect("write java fixture");

    let recording = start_recording();

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
        .build()
        .expect("build executable component");
    component
        .attach_cpu_profiler()
        .expect("attach cpu profiler");
    let component_id = component.identifier().to_string();

    component.start().await.expect("start component");
    await_jvm_running(&marker_path, &mut component).await;
    tokio::time::sleep(SAMPLING_WINDOW).await;
    component.stop().await.expect("stop component");

    let folded = recording
        .recorded
        .folded_for(&component_id)
        .expect("folded stacks were not reported");
    assert!(!folded.trim().is_empty(), "folded stacks were empty");
    let _ = std::fs::remove_file(&fixture_path);
    let _ = std::fs::remove_file(&marker_path);
}
