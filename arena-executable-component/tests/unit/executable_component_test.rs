use arena::component::RunnableComponent;
use arena_executable_component::executable_component::{merged_profile_env_value, ExecutableComponent};
use arena_profile::{ProfileEnvVar, ShutdownSignal};
use std::ffi::OsString;
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

#[test]
fn merged_profile_env_value_without_a_caller_value_returns_ours() {
    let entry = ProfileEnvVar::path_joined("PYTHONPATH", "/scratch/arena");

    let merged = merged_profile_env_value(&entry, None).expect("merge");

    assert_eq!(merged, OsString::from("/scratch/arena"));
}

#[test]
fn merged_profile_env_value_with_an_empty_caller_value_returns_ours() {
    let entry = ProfileEnvVar::path_joined("PYTHONPATH", "/scratch/arena");

    let merged = merged_profile_env_value(&entry, Some("")).expect("merge");

    assert_eq!(merged, OsString::from("/scratch/arena"));
}

#[test]
fn merged_profile_env_value_space_merge_keeps_the_caller_value_first() {
    let entry = ProfileEnvVar::space_joined("JAVA_TOOL_OPTIONS", "-agentpath:/scratch/agent.so");

    let merged = merged_profile_env_value(&entry, Some("-Xmx512m")).expect("merge");

    assert_eq!(merged, OsString::from("-Xmx512m -agentpath:/scratch/agent.so"));
}

#[test]
fn merged_profile_env_value_path_merge_appends_ours_behind_every_caller_entry() {
    let entry = ProfileEnvVar::path_joined("PYTHONPATH", "/scratch/arena");
    let caller = std::env::join_paths(["/app/src", "/app/deps"])
        .expect("caller path list")
        .into_string()
        .expect("utf8");

    let merged = merged_profile_env_value(&entry, Some(&caller)).expect("merge");

    let entries: Vec<std::path::PathBuf> = std::env::split_paths(&merged).collect();
    assert_eq!(
        entries,
        vec![
            std::path::PathBuf::from("/app/src"),
            std::path::PathBuf::from("/app/deps"),
            std::path::PathBuf::from("/scratch/arena"),
        ]
    );
}

#[test]
fn merged_profile_env_value_path_merge_never_joins_with_a_space() {
    let entry = ProfileEnvVar::path_joined("PYTHONPATH", "/scratch/arena");

    let merged = merged_profile_env_value(&entry, Some("/app/src"))
        .expect("merge")
        .into_string()
        .expect("utf8");

    assert!(!merged.contains(' '), "got: {merged}");
    assert_eq!(std::env::split_paths(&merged).count(), 2);
}
