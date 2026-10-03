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
            "--",
            "/usr/bin/python3",
            "-c",
            "pass",
        ]
    );
}

#[cfg(unix)]
#[test]
fn collect_success_status_returns_folded_path() {
    let mut child = Command::new("true").spawn().expect("spawn true");
    let _ = child.wait();
    let folded_path = PathBuf::from("/tmp/arena-profile-pyspy-test-collect-success.folded");
    let state = WrapState::PySpy { folded_path: folded_path.clone() };

    let result = PySpySampler.collect(state, &mut child, Duration::from_secs(5));

    assert_eq!(result.unwrap(), folded_path);
}

#[cfg(unix)]
#[test]
fn collect_non_success_status_returns_finish_error() {
    let mut child = Command::new("false").spawn().expect("spawn false");
    let _ = child.wait();
    let state = WrapState::PySpy {
        folded_path: PathBuf::from("/tmp/arena-profile-pyspy-test-collect-fail.folded"),
    };

    let result = PySpySampler.collect(state, &mut child, Duration::from_secs(5));

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
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
