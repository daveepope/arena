use arena::component::RunnableComponent;
use arena_executable_component::executable_component::ExecutableComponent;
use std::time::Duration;

#[cfg(unix)]
fn sleeper_command(pid_file: &str) -> (&'static str, Vec<String>) {
    (
        "/bin/sh",
        vec![
            "-c".to_string(),
            format!("echo $$ > {pid_file} && sleep 30"),
        ],
    )
}

#[cfg(windows)]
fn sleeper_command(pid_file: &str) -> (&'static str, Vec<String>) {
    (
        "powershell",
        vec![
            "-NoProfile".to_string(),
            "-Command".to_string(),
            format!(
                "$PID | Out-File -FilePath '{pid_file}' -Encoding ascii; Start-Sleep -Seconds 30"
            ),
        ],
    )
}

#[cfg(unix)]
fn process_is_running(pid: &str) -> bool {
    std::process::Command::new("kill")
        .args(["-0", pid])
        .status()
        .expect("run kill -0")
        .success()
}

#[cfg(windows)]
fn process_is_running(pid: &str) -> bool {
    let command = format!("(Get-Process -Id {pid} -ErrorAction SilentlyContinue) -ne $null");
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", command.as_str()])
        .output()
        .expect("run Get-Process");
    String::from_utf8_lossy(&output.stdout).trim() == "True"
}

#[tokio::test]
async fn stop_no_profiling_configured_kills_child_process() {
    let pid_file = std::env::temp_dir().join(format!(
        "arena-executable-component-lifecycle-test-{}.pid",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&pid_file);
    let pid_file_str = pid_file.to_string_lossy().into_owned();

    let (program, args) = sleeper_command(&pid_file_str);
    let mut builder = ExecutableComponent::builder("lifecycle-test").with_executable_path(program);
    for (index, arg) in args.into_iter().enumerate() {
        builder = builder.with_runtime_arg(format!("arg{index}"), arg);
    }
    let mut component = builder.build().expect("build executable component");

    component.start().await.expect("start component");

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

    component.stop().await.expect("stop component");

    assert!(
        !process_is_running(&child_pid),
        "child process should have been killed by stop()"
    );
    let _ = std::fs::remove_file(&pid_file);
}
