use arena_profile::backend::dotnet_trace::{chromium_trace_to_folded, record_args, DotnetTraceSampler};
use arena_profile::backend::ScratchDir;
use arena_profile::wrapped::{WrapState, WrappingSampler};
use arena_profile::{CpuProfileError, LaunchRequest, DEFAULT_CPU_PROFILE_LIMIT_BYTES};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

#[test]
fn record_args_builds_dotnet_trace_collect_invocation() {
    let nettrace_path = PathBuf::from("/tmp/arena-profile-dotnet-trace-test-sample.nettrace");
    let request = LaunchRequest {
        program: PathBuf::from("/usr/bin/dotnet"),
        args: vec!["app.dll".to_string()],
    };

    let args = record_args(&nettrace_path, &request);

    assert_eq!(
        args,
        vec![
            "collect",
            "-o",
            "/tmp/arena-profile-dotnet-trace-test-sample.nettrace",
            "--",
            "/usr/bin/dotnet",
            "app.dll",
        ]
    );
}

#[cfg(unix)]
#[test]
fn collect_missing_trace_file_returns_finish_error() {
    // dotnet-trace's own exit code mirrors the traced process's exit code, not success of the
    // trace itself (per its own --help text), so collect() must not treat a non-zero exit from
    // the wrapping_child as failure. The real failure signal is the trace file never being written.
    let mut child = Command::new("false").spawn().expect("spawn false");
    let _ = child.wait();
    let scratch = ScratchDir::create("arena-profile-dotnet-trace-test").expect("scratch");
    let nettrace_path = scratch.file_path("trace.nettrace");
    let state = WrapState::DotnetTrace { scratch, nettrace_path };

    let result = DotnetTraceSampler.collect(
        state,
        &mut child,
        Duration::from_secs(5),
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    );

    assert!(matches!(result, Err(CpuProfileError::Finish(_))));
}

#[cfg(unix)]
#[test]
fn collect_non_zero_exit_status_with_trace_file_present_is_not_treated_as_failure() {
    // Regression test: dotnet-trace reports the traced process's exit code as its own, so a
    // non-zero status from the wrapping child must not short-circuit collect() by itself.
    let mut child = Command::new("false").spawn().expect("spawn false");
    let _ = child.wait();
    let scratch = ScratchDir::create("arena-profile-dotnet-trace-test").expect("scratch");
    let (nettrace_path, _file) = scratch.create_file("trace.nettrace").expect("create trace file");
    std::fs::write(&nettrace_path, b"not a real trace file, it only needs to exist")
        .expect("write the stand-in trace file");
    let state = WrapState::DotnetTrace { scratch, nettrace_path };

    let result = DotnetTraceSampler.collect(
        state,
        &mut child,
        Duration::from_secs(5),
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    );

    // Past the exit-status/file-exists checks, collect() proceeds to `dotnet-trace convert`,
    // which fails either because the tool isn't resolvable (MissingBinary) or because it
    // rejects the placeholder file's contents (Finish) — either way, the important assertion
    // is that it is not a "non-zero exit status" short-circuit.
    match result {
        Err(CpuProfileError::MissingBinary { binary: "dotnet-trace", .. }) => {}
        Err(CpuProfileError::Finish(ref msg)) if msg.contains("convert") => {}
        other => panic!("expected collect() to proceed to dotnet-trace convert, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
#[should_panic(expected = "DotnetTraceSampler::collect given a non-dotnet-trace WrapState")]
fn collect_wrong_wrap_state_panics_unreachable() {
    let mut child = Command::new("true").spawn().expect("spawn true");
    let _ = child.wait();

    let scratch = ScratchDir::create("arena-profile-dotnet-trace-test").expect("scratch");
    let data_path = scratch.file_path("perf.data");
    let _ = DotnetTraceSampler.collect(
        WrapState::Perf { scratch, data_path },
        &mut child,
        Duration::from_secs(5),
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    );
}

const SAMPLE_CHROMIUM_TRACE: &str = r#"{
  "traceEvents": [
    { "name": "root", "cat": "sampleEvent", "ph": "B", "ts": 0.0, "pid": 0, "tid": 1, "sf": 0 },
    { "name": "outer", "cat": "sampleEvent", "ph": "B", "ts": 0.0, "pid": 0, "tid": 1, "sf": 1 },
    { "name": "inner", "cat": "sampleEvent", "ph": "B", "ts": 0.0, "pid": 0, "tid": 1, "sf": 2 },
    { "name": "inner", "cat": "sampleEvent", "ph": "E", "ts": 10.0, "pid": 0, "tid": 1, "sf": 2 },
    { "name": "outer", "cat": "sampleEvent", "ph": "E", "ts": 15.0, "pid": 0, "tid": 1, "sf": 1 },
    { "name": "root", "cat": "sampleEvent", "ph": "E", "ts": 15.0, "pid": 0, "tid": 1, "sf": 0 }
  ],
  "stackFrames": {
    "0": { "name": "root", "category": "" },
    "1": { "name": "outer", "category": "", "parent": 0 },
    "2": { "name": "inner", "category": "", "parent": 1 }
  }
}"#;

#[test]
fn chromium_trace_to_folded_reconstructs_stacks_with_duration_weights() {
    let folded = chromium_trace_to_folded(SAMPLE_CHROMIUM_TRACE).expect("fold sample trace");

    assert_eq!(folded, "root;outer 5\nroot;outer;inner 10\n");
}

#[test]
fn chromium_trace_to_folded_empty_trace_returns_no_stacks() {
    let empty = r#"{"traceEvents": [], "stackFrames": {}}"#;

    let result = chromium_trace_to_folded(empty);

    assert_eq!(result.unwrap(), "");
}

#[test]
fn chromium_trace_to_folded_invalid_json_returns_err() {
    let result = chromium_trace_to_folded("not json");

    assert!(result.is_err());
}
