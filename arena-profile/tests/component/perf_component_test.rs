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
fn prepare_cpu_profile_real_perf_wrapping_busy_loop_reports_folded_stacks() {
    let component_id = "perf-component-test";
    let recording = start_recording();

    let marker = marker_path("perf");
    let request = LaunchRequest {
        program: subject_path("tools/profile_subjects/busy_loop_native").into(),
        args: vec![
            PASS_MILLIS.to_string(),
            marker.to_string_lossy().into_owned(),
        ],
    };

    let prepared =
        prepare_cpu_profile(CpuProfilerBackend::Perf, request, component_id, DEFAULT_CPU_PROFILE_LIMIT_BYTES).expect("prepare perf profile");
    let PreparedLaunch::Wrapped { program, args, session } = prepared else {
        panic!("expected Wrapped variant for Perf backend");
    };

    let mut wrapping_child = Command::new(&program)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn perf wrapping process");

    if let Err(reason) = await_completed_pass(&marker, &mut wrapping_child) {
        let _ = wrapping_child.kill();
        let _ = wrapping_child.wait();
        let _ = std::fs::remove_file(&marker);
        panic!("{reason}");
    }

    session.finish(&mut wrapping_child).expect("finish perf profile");
    let _ = wrapping_child.wait();
    let _ = std::fs::remove_file(&marker);

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
}
