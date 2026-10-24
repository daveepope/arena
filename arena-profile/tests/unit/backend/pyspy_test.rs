use arena_profile::backend::pyspy::{record_args, PySpySampler};
use arena_profile::wrapped::{WrapState, WrappingSampler};
use arena_profile::backend::ScratchDir;
use arena_profile::{CpuProfileError, LaunchRequest, DEFAULT_CPU_PROFILE_LIMIT_BYTES};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

#[test]
fn record_args_builds_py_spy_record_invocation() {
    let folded_path = PathBuf::from("/tmp/arena-profile-pyspy-test-sample.folded");
    let request = LaunchRequest {
        program: PathBuf::from("/usr/bin/python3"),
        args: vec!["-c".to_string(), "pass".to_string()],
    };

    let args = record_args(&folded_path, &request);

    assert_eq!(
        args,
        vec![
            "record",
            "-o",
            "/tmp/arena-profile-pyspy-test-sample.folded",
            "--format",
            "raw",
            "--idle",
            "--",
            "/usr/bin/python3",
            "-c",
            "pass",
        ]
    );
}

#[cfg(unix)]
#[test]
fn collect_missing_folded_file_returns_finish_error() {
    // py-spy's own exit status reflects how our SIGINT was delivered (often a raw signal
    // termination, not a caught exit(0)), not whether sampling succeeded, so collect() must
    // not treat a non-zero exit from the wrapping_child as failure. The real failure signal
    // is the folded stacks file never being written.
    let mut child = Command::new("false").spawn().expect("spawn false");
    let _ = child.wait();
    let scratch = ScratchDir::create("arena-profile-pyspy-test").expect("scratch");
    let folded_path = scratch.file_path("profile.folded");
    let state = WrapState::PySpy { scratch, folded_path };

    let result = PySpySampler.collect(
        state,
        &mut child,
        Duration::from_secs(5),
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    );

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
}

#[cfg(unix)]
#[test]
fn collect_non_zero_exit_status_with_folded_file_present_is_not_treated_as_failure() {
    // Regression test: a SIGINT'd py-spy commonly reports a non-success exit status, so that
    // status must not short-circuit collect() by itself.
    let mut child = Command::new("false").spawn().expect("spawn false");
    let _ = child.wait();
    let scratch = ScratchDir::create("arena-profile-pyspy-test").expect("scratch");
    let (folded_path, _file) = scratch.create_file("profile.folded").expect("create folded file");
    std::fs::write(&folded_path, b"main;compute 3\n").expect("write folded stacks");
    let state = WrapState::PySpy { scratch, folded_path };

    let result = PySpySampler.collect(
        state,
        &mut child,
        Duration::from_secs(5),
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    );

    assert_eq!(result.unwrap(), "main;compute 3\n".to_string());
}

#[cfg(unix)]
#[test]
#[should_panic(expected = "PySpySampler::collect given a non-py-spy WrapState")]
fn collect_wrong_wrap_state_panics_unreachable() {
    let mut child = Command::new("true").spawn().expect("spawn true");
    let _ = child.wait();

    let scratch = ScratchDir::create("arena-profile-pyspy-test").expect("scratch");
    let data_path = scratch.file_path("perf.data");
    let _ = PySpySampler.collect(
        WrapState::Perf { scratch, data_path },
        &mut child,
        Duration::from_secs(5),
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    );
}

#[cfg(unix)]
#[test]
fn collect_clean_exit_without_folded_file_returns_finish_error() {
    let mut child = Command::new("true").spawn().expect("spawn true");
    let _ = child.wait();
    let scratch = ScratchDir::create("arena-profile-pyspy-test").expect("scratch");
    let folded_path = scratch.file_path("profile.folded");
    let state = WrapState::PySpy { scratch, folded_path };

    let result = PySpySampler.collect(
        state,
        &mut child,
        Duration::from_secs(5),
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    );

    match result {
        Err(CpuProfileError::Finish(message)) => assert!(
            message.contains("wrote no folded stacks"),
            "the failure must say py-spy wrote nothing, got: {message}"
        ),
        other => panic!("a missing profile must fault, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn collect_interrupted_exit_with_empty_folded_file_returns_finish_error() {
    let scratch = ScratchDir::create("arena-profile-pyspy-test").expect("scratch");
    let (folded_path, _file) = scratch.create_file("profile.folded").expect("create folded file");
    let mut child = Command::new("sleep").arg("30").spawn().expect("spawn sleep");
    let _ = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("interrupt sleep");
    let state = WrapState::PySpy { scratch, folded_path };

    let result = PySpySampler.collect(
        state,
        &mut child,
        Duration::from_secs(5),
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    );

    match result {
        Err(CpuProfileError::Finish(message)) => assert!(
            message.contains("recorded no samples"),
            "the failure must say py-spy recorded nothing, got: {message}"
        ),
        other => panic!("an empty profile must fault, got {other:?}"),
    }
}
