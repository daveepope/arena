use arena_profile::backend::{
    binary_on_path, clear_stale_runfiles_manifest_env, map_spawn_error, sampled_nothing,
    signal_interrupt, ScratchDir,
};
use arena_profile::{wait_bounded, CpuProfileError};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

static RUNFILES_MANIFEST_ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn wait_bounded_child_outlives_budget_kills_and_returns_timed_out() {
    let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

    let result = wait_bounded(&mut child, Duration::from_millis(100));

    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::TimedOut);
    assert!(child.try_wait().unwrap().is_some());
}


#[cfg(unix)]
#[test]
fn resolve_binary_fallback_on_path_returns_ok() {
    let _guard = RUNFILES_MANIFEST_ENV_LOCK.lock().unwrap();
    let result = arena_profile::backend::resolve_binary(&[], "sh", "sh", "install sh");

    assert_eq!(result.unwrap(), "sh");
}

#[test]
fn resolve_binary_not_on_path_returns_missing_binary() {
    let _guard = RUNFILES_MANIFEST_ENV_LOCK.lock().unwrap();
    let result = arena_profile::backend::resolve_binary(
        &[],
        "arena-profile-nonexistent-binary",
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
    let _guard = RUNFILES_MANIFEST_ENV_LOCK.lock().unwrap();
    std::env::set_var("RUNFILES_MANIFEST_FILE", "/nonexistent/arena-profile-fake-manifest");

    clear_stale_runfiles_manifest_env();

    assert!(std::env::var_os("RUNFILES_MANIFEST_FILE").is_none());
}

#[test]
fn clear_stale_runfiles_manifest_env_existing_path_leaves_var_set() {
    let _guard = RUNFILES_MANIFEST_ENV_LOCK.lock().unwrap();
    let real_file = std::env::temp_dir().join(format!(
        "arena-profile-runfiles-manifest-test-{}.txt",
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

#[cfg(unix)]
#[test]
fn binary_on_path_absolute_existing_path_returns_true() {
    assert!(binary_on_path("/bin/sh"));
}

#[test]
fn binary_on_path_absolute_missing_path_returns_false() {
    assert!(!binary_on_path("/nonexistent/arena-profile-fake-binary"));
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

#[cfg(unix)]
#[test]
fn signal_interrupt_running_child_returns_ok_and_interrupts() {
    let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

    let result = signal_interrupt(&mut child);

    assert!(result.is_ok());
    std::thread::sleep(Duration::from_millis(100));
    assert!(child.try_wait().unwrap().is_some());
}

#[cfg(unix)]
#[test]
fn signal_interrupt_already_exited_child_returns_ok_without_signaling() {
    let mut child = Command::new("true").spawn().expect("spawn true");
    let _ = child.wait();

    let result = signal_interrupt(&mut child);

    assert!(result.is_ok());
}

#[test]
fn sampled_nothing_empty_file_returns_true() {
    let path = std::env::temp_dir().join(format!("arena-profile-sampled-nothing-empty-{}", std::process::id()));
    std::fs::write(&path, b"").expect("write empty profile");

    assert!(sampled_nothing(&path));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn sampled_nothing_populated_file_returns_false() {
    let path = std::env::temp_dir().join(format!("arena-profile-sampled-nothing-full-{}", std::process::id()));
    std::fs::write(&path, b"main;run 1\n").expect("write profile");

    assert!(!sampled_nothing(&path));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn sampled_nothing_missing_file_returns_false() {
    let path = std::env::temp_dir().join(format!("arena-profile-sampled-nothing-absent-{}", std::process::id()));
    let _ = std::fs::remove_file(&path);

    assert!(!sampled_nothing(&path));
}

#[cfg(unix)]
fn mode_of(path: &std::path::Path) -> u32 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).expect("metadata").mode() & 0o777
}

#[test]
#[cfg(unix)]
fn scratch_dir_created_file_is_private_inside_a_private_directory() {
    let scratch = ScratchDir::create("arena-profile-scratch-test").expect("create scratch dir");

    let (path, _file) = scratch.create_file("sitecustomize.py").expect("create file");

    assert_eq!(mode_of(scratch.path()), 0o700);
    assert_eq!(mode_of(&path), 0o600);
}

#[test]
fn scratch_dir_create_file_twice_for_one_name_refuses_the_second() {
    let scratch = ScratchDir::create("arena-profile-scratch-test").expect("create scratch dir");
    let _first = scratch.create_file("output.folded").expect("create file");

    let second = scratch.create_file("output.folded");

    assert!(matches!(second, Err(CpuProfileError::Launch(_))), "a pre-existing name must be refused");
}

#[test]
fn scratch_dir_two_runs_never_share_a_directory() {
    let first = ScratchDir::create("arena-profile-scratch-test").expect("first");
    let second = ScratchDir::create("arena-profile-scratch-test").expect("second");

    assert_ne!(first.path(), second.path());
}

#[test]
fn scratch_dir_dropped_removes_the_directory_and_its_contents() {
    let scratch = ScratchDir::create("arena-profile-scratch-test").expect("create scratch dir");
    let held = scratch.path().to_path_buf();
    scratch.create_file("output.folded").expect("create file");

    drop(scratch);

    assert!(!held.exists(), "{} survived the guard", held.display());
}
