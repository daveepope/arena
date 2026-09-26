use arena::component::RunnableComponent;
use arena_executable_component::executable_component::ExecutableComponent;
use arena_profile::ShutdownSignal;
use std::process::Command;

#[test]
fn log_line_every_severity_marker_does_not_panic() {
    for line in [
        "2024-01-01 ERROR something broke",
        "2024-01-01 WARN heads up",
        "2024-01-01 DEBUG detail",
        "2024-01-01 TRACE fine detail",
        "2024-01-01 INFO normal",
    ] {
        ExecutableComponent::log_line("exec-log-line", line);
    }
}

#[cfg(unix)]
#[test]
fn signal_terminate_running_child_terminates_it() {
    let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

    let result = ExecutableComponent::signal_terminate(&mut child);

    assert!(result.is_ok());
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(child.try_wait().unwrap().is_some());
}

#[cfg(unix)]
#[test]
fn signal_terminate_already_exited_child_returns_ok() {
    let mut child = Command::new("true").spawn().expect("spawn true");
    let _ = child.wait();

    let result = ExecutableComponent::signal_terminate(&mut child);

    assert!(result.is_ok());
}

#[cfg(unix)]
#[test]
fn graceful_then_force_kill_kill_signal_stops_child() {
    let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

    ExecutableComponent::graceful_then_force_kill(&mut child, ShutdownSignal::Kill, "exec-kill");

    assert!(child.try_wait().unwrap().is_some());
}

#[cfg(unix)]
#[test]
fn graceful_then_force_kill_terminate_signal_stops_child() {
    let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

    ExecutableComponent::graceful_then_force_kill(
        &mut child,
        ShutdownSignal::Terminate,
        "exec-terminate",
    );

    assert!(child.try_wait().unwrap().is_some());
}

#[tokio::test]
async fn start_missing_program_returns_fault() {
    let mut component = ExecutableComponent::builder("exec-missing-program")
        .with_executable_path("/nonexistent/arena-executable-component-absent-binary")
        .build()
        .expect("build executable component");

    let fault = component.start().await.err().expect("missing program must fault");

    assert!(fault.id.contains("exec-missing-program"));
    assert!(fault.message.contains("spawn failed"));
}

#[tokio::test]
async fn stop_never_started_component_returns_ok() {
    let mut component = ExecutableComponent::builder("exec-never-started")
        .build()
        .expect("build executable component");

    assert!(component.stop().await.is_ok());
}
