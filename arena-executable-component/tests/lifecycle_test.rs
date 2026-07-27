use arena::component::RunnableComponent;
use arena_executable_component::executable_component::ExecutableComponent;
use std::time::Duration;

#[tokio::test]
async fn stop_no_profiling_configured_kills_child_process() {
    let pid_file = std::env::temp_dir().join(format!(
        "arena-executable-component-lifecycle-test-{}.pid",
        std::process::id()
    ));
    let pid_file_str = pid_file.to_string_lossy().into_owned();

    let mut component = ExecutableComponent::builder("lifecycle-test")
        .with_executable_path("/bin/sh")
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", format!("echo $$ > {pid_file_str} && sleep 30"))
        .build();

    component.start().await;

    let mut child_pid = None;
    for _ in 0..50 {
        if let Ok(contents) = std::fs::read_to_string(&pid_file) {
            let trimmed = contents.trim();
            if !trimmed.is_empty() {
                child_pid = Some(trimmed.to_string());
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let child_pid = child_pid.expect("child process should have written its pid");

    component.stop().await;

    let still_running = std::process::Command::new("kill")
        .args(["-0", &child_pid])
        .status()
        .expect("run kill -0")
        .success();

    assert!(!still_running, "child process should have been killed by stop()");
    let _ = std::fs::remove_file(&pid_file);
}
