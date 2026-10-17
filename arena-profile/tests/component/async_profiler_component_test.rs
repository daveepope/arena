#[path = "../support/profile_subjects.rs"]
mod profile_subjects;
#[path = "../support/recorded_profiles.rs"]
mod recorded_profiles;

use arena_profile::DEFAULT_CPU_PROFILE_LIMIT_BYTES;
use arena_profile::{prepare_cpu_profile, CpuProfilerBackend, LaunchRequest, PreparedLaunch, ShutdownSignal};
use arena::lifecycle::RunnableState;
use profile_subjects::{await_completed_pass, marker_path, subject_path, PASS_MILLIS};
use recorded_profiles::start_recording;
use std::process::{Command, Stdio};

#[test]
fn prepare_cpu_profile_real_async_profiler_agent_flag_against_jvm_busy_loop_reports_folded_stacks() {
    let component_id = "async_profiler-component-test";
    let recording = start_recording();

    let marker = marker_path("async-profiler");
    let request = LaunchRequest {
        program: "java".into(),
        args: vec![
            subject_path("tools/profile_subjects/BusyLoop.java"),
            PASS_MILLIS.to_string(),
            marker.to_string_lossy().into_owned(),
        ],
    };

    let prepared = prepare_cpu_profile(CpuProfilerBackend::AsyncProfiler, request, component_id, DEFAULT_CPU_PROFILE_LIMIT_BYTES)
        .expect("prepare async-profiler profile");
    let PreparedLaunch::ArgsAugmented { args, shutdown_signal, session } = prepared else {
        panic!("expected ArgsAugmented variant for AsyncProfiler backend");
    };
    assert_eq!(shutdown_signal, ShutdownSignal::Terminate);

    let mut jvm = Command::new("java")
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jvm with agent flag");

    if let Err(reason) = await_completed_pass(&marker, &mut jvm) {
        let _ = jvm.kill();
        let _ = jvm.wait();
        let _ = std::fs::remove_file(&marker);
        panic!("{reason}");
    }

    let status = Command::new("kill")
        .args(["-TERM", &jvm.id().to_string()])
        .status()
        .expect("send SIGTERM to jvm");
    assert!(status.success());

    session.finish(&mut jvm).expect("finish async-profiler profile");

    let folded = recording
        .recorded
        .folded_for(component_id)
        .expect("folded stacks were not reported");
    assert!(
        folded.contains("bar"),
        "the subject's hotspot was not sampled, got: {folded}"
    );
    assert!(recording
        .recorded
        .states_for(component_id)
        .contains(&RunnableState::Stopped));
    let _ = std::fs::remove_file(&marker);
}
