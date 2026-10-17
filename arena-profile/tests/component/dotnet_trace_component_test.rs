#[path = "../support/profile_subjects.rs"]
mod profile_subjects;
#[path = "../support/recorded_profiles.rs"]
mod recorded_profiles;

use arena_profile::DEFAULT_CPU_PROFILE_LIMIT_BYTES;
use arena_profile::{prepare_cpu_profile, CpuProfilerBackend, LaunchRequest, PreparedLaunch};
use arena::lifecycle::RunnableState;
use profile_subjects::{await_completed_pass, marker_path, subject_path, PASS_MILLIS};
use recorded_profiles::start_recording;
use std::process::{Command, Stdio};

#[test]
fn prepare_cpu_profile_real_dotnet_trace_wrapping_busy_loop_reports_folded_stacks() {
    let component_id = "dotnet_trace-component-test";
    let recording = start_recording();
    let marker = marker_path("dotnet-trace");
    let request = LaunchRequest {
        program: subject_path("tools/profile_subjects/busy_loop_dotnet/net8.0/busy_loop_dotnet.dll.sh")
            .into(),
        args: vec![
            PASS_MILLIS.to_string(),
            marker.to_string_lossy().into_owned(),
        ],
    };

    let prepared = prepare_cpu_profile(CpuProfilerBackend::DotnetTrace, request, component_id, DEFAULT_CPU_PROFILE_LIMIT_BYTES)
        .expect("prepare dotnet-trace profile");
    let PreparedLaunch::Wrapped { program, args, session } = prepared else {
        panic!("expected Wrapped variant for DotnetTrace backend");
    };

    let mut wrapping_child = Command::new(&program)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn dotnet-trace wrapping process");

    if let Err(reason) = await_completed_pass(&marker, &mut wrapping_child) {
        let _ = wrapping_child.kill();
        let _ = wrapping_child.wait();
        let _ = std::fs::remove_file(&marker);
        panic!("{reason}");
    }

    session.finish(&mut wrapping_child).expect("finish dotnet-trace profile");
    let _ = wrapping_child.wait();

    let folded = recording
        .recorded
        .folded_for(component_id)
        .expect("folded stacks were not reported");
    assert!(
        folded.contains("BusyLoop"),
        "the subject was not sampled, got: {folded}"
    );
    assert!(recording
        .recorded
        .states_for(component_id)
        .contains(&RunnableState::Stopped));
    let _ = std::fs::remove_file(&marker);
}
