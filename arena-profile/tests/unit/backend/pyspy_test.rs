use arena_profile::backend::pyspy::{record_args, PySpySampler};
use arena_profile::wrapped::{WrapState, WrappingSampler};
use arena_profile::{CpuProfileError, LaunchRequest};
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
    let state = WrapState::PySpy {
        folded_path: PathBuf::from("/tmp/arena-profile-pyspy-test-collect-missing.folded"),
    };

    let result = PySpySampler.collect(state, &mut child, Duration::from_secs(5));

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
}

#[cfg(unix)]
#[test]
fn collect_non_zero_exit_status_with_folded_file_present_is_not_treated_as_failure() {
    // Regression test: a SIGINT'd py-spy commonly reports a non-success exit status, so that
    // status must not short-circuit collect() by itself.
    let mut child = Command::new("false").spawn().expect("spawn false");
    let _ = child.wait();
    let folded_path = PathBuf::from("/tmp/arena-profile-pyspy-test-collect-present.folded");
    std::fs::write(&folded_path, b"not real folded stacks, just needs to exist")
        .expect("write placeholder folded stacks file");
    let state = WrapState::PySpy { folded_path: folded_path.clone() };

    let result = PySpySampler.collect(state, &mut child, Duration::from_secs(5));

    assert_eq!(result.unwrap(), folded_path);
    let _ = std::fs::remove_file(&folded_path);
}

#[cfg(unix)]
#[test]
#[should_panic(expected = "PySpySampler::collect given a non-py-spy WrapState")]
fn collect_wrong_wrap_state_panics_unreachable() {
    let mut child = Command::new("true").spawn().expect("spawn true");
    let _ = child.wait();

    let _ = PySpySampler.collect(
        WrapState::Perf { data_path: PathBuf::from("/tmp/unused") },
        &mut child,
        Duration::from_secs(5),
    );
}
