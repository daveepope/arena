use arena_profile::backend::perf::{record_args, PerfSampler};
use arena_profile::wrapped::{WrapState, WrappingSampler};
use arena_profile::LaunchRequest;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

#[test]
fn record_args_builds_perf_record_invocation() {
    let data_path = PathBuf::from("/tmp/arena-profile-perf-test-sample.data");
    let request = LaunchRequest {
        program: PathBuf::from("/usr/bin/myapp"),
        args: vec!["--port".to_string(), "8080".to_string()],
    };

    let args = record_args(&data_path, &request);

    assert_eq!(
        args,
        vec![
            "record",
            "-g",
            "-o",
            "/tmp/arena-profile-perf-test-sample.data",
            "--",
            "/usr/bin/myapp",
            "--port",
            "8080",
        ]
    );
}

#[cfg(unix)]
#[test]
#[should_panic(expected = "PerfSampler::collect given a non-perf WrapState")]
fn collect_wrong_wrap_state_panics_unreachable() {
    let mut child = Command::new("true").spawn().expect("spawn true");
    let _ = child.wait();

    let _ = PerfSampler.collect(
        WrapState::PySpy { folded_path: PathBuf::from("/tmp/unused") },
        &mut child,
        Duration::from_secs(5),
    );
}
