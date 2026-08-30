use arena_profiler::backend::{
    binary_on_path, clear_stale_runfiles_manifest_env, map_spawn_error, resolve_binary,
    scratch_path, signal_interrupt, wait_bounded,
};
use arena_profiler::CpuProfileError;
use std::process::Command;
use std::time::Duration;

#[test]
fn scratch_path_called_twice_returns_distinct_paths() {
    let first = scratch_path("prefix", "ext");
    let second = scratch_path("prefix", "ext");

    assert_ne!(first, second);
}

#[test]
fn resolve_binary_fallback_on_path_returns_ok() {
    let result = resolve_binary(&[], "sh", "sh", "install sh");

    assert_eq!(result.unwrap(), "sh");
}

#[test]
fn resolve_binary_not_on_path_returns_missing_binary() {
    let result = resolve_binary(
        &[],
        "arena-profiler-nonexistent-binary",
        "fake-tool",
        "install fake-tool",
    );

    assert!(matches!(
        result,
        Err(CpuProfileError::MissingBinary { binary: "fake-tool", .. })
    ));
}

#[test]
fn clear_stale_runfiles_manifest_env_nonexistent_path_removes_var() {
    std::env::set_var("RUNFILES_MANIFEST_FILE", "/nonexistent/arena-profiler-fake-manifest");

    clear_stale_runfiles_manifest_env();

    assert!(std::env::var_os("RUNFILES_MANIFEST_FILE").is_none());
}

#[test]
fn clear_stale_runfiles_manifest_env_existing_path_leaves_var_set() {
    let real_file = std::env::temp_dir().join(format!(
        "arena-profiler-runfiles-manifest-test-{}.txt",
        std::process::id()
    ));
    std::fs::write(&real_file, "").expect("write manifest fixture");
    std::env::set_var("RUNFILES_MANIFEST_FILE", &real_file);

    clear_stale_runfiles_manifest_env();

    assert_eq!(
        std::env::var_os("RUNFILES_MANIFEST_FILE").as_deref(),
        Some(real_file.as_os_str())
    );
    std::env::remove_var("RUNFILES_MANIFEST_FILE");
    let _ = std::fs::remove_file(&real_file);
}

#[test]
fn binary_on_path_absolute_existing_path_returns_true() {
    assert!(binary_on_path("/bin/sh"));
}

#[test]
fn binary_on_path_absolute_missing_path_returns_false() {
    assert!(!binary_on_path("/nonexistent/arena-profiler-fake-binary"));
}

#[test]
fn map_spawn_error_not_found_returns_missing_binary() {
    let e = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");

    let result = map_spawn_error("perf", "install perf", e);

    assert!(matches!(result, CpuProfileError::MissingBinary { binary: "perf", .. }));
}

#[test]
fn map_spawn_error_other_kind_returns_spawn() {
    let e = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");

    let result = map_spawn_error("perf", "install perf", e);

    assert!(matches!(result, CpuProfileError::Spawn(_)));
}

#[test]
fn signal_interrupt_running_child_returns_ok_and_interrupts() {
    let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

    let result = signal_interrupt(&mut child);

    assert!(result.is_ok());
    std::thread::sleep(Duration::from_millis(100));
    assert!(child.try_wait().unwrap().is_some());
}

#[test]
fn signal_interrupt_already_exited_child_returns_ok_without_signaling() {
    let mut child = Command::new("true").spawn().expect("spawn true");
    let _ = child.wait();

    let result = signal_interrupt(&mut child);

    assert!(result.is_ok());
}

#[test]
fn wait_bounded_child_outlives_budget_kills_and_returns_timed_out() {
    let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

    let result = wait_bounded(&mut child, Duration::from_millis(100));

    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::TimedOut);
    assert!(child.try_wait().unwrap().is_some());
}

#[test]
fn wait_bounded_child_exits_within_budget_returns_status() {
    let mut child = Command::new("true").spawn().expect("spawn true");

    let status = wait_bounded(&mut child, Duration::from_secs(5)).expect("wait for child");

    assert!(status.success());
}
