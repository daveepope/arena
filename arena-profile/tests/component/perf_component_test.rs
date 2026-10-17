#[path = "../support/recorded_profiles.rs"]
mod recorded_profiles;

use arena_profile::{prepare_cpu_profile, CpuProfilerBackend, LaunchRequest, PreparedLaunch};
use arena::lifecycle::RunnableState;
use recorded_profiles::start_recording;
use std::process::{Command, Stdio};
use std::time::Duration;

#[test]
fn prepare_cpu_profile_real_perf_wrapping_busy_loop_reports_folded_stacks() {
    let component_id = "perf-component-test";
    let recording = start_recording();

    let request = LaunchRequest {
        program: "sh".into(),
        args: vec![
            "-c".to_string(),
            "i=0; while true; do i=$((i+1)); done".to_string(),
        ],
    };

    let prepared =
        prepare_cpu_profile(CpuProfilerBackend::Perf, request, component_id).expect("prepare perf profile");
    let PreparedLaunch::Wrapped { program, args, session } = prepared else {
        panic!("expected Wrapped variant for Perf backend");
    };

    let mut wrapping_child = Command::new(&program)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn perf wrapping process");

    std::thread::sleep(Duration::from_millis(500));

    session.finish(&mut wrapping_child).expect("finish perf profile");
    let _ = wrapping_child.wait();

    let folded = recording
        .recorded
        .folded_for(component_id)
        .expect("folded stacks were not reported");
    assert!(!folded.trim().is_empty(), "folded stacks were empty");
    assert!(recording
        .recorded
        .states_for(component_id)
        .contains(&RunnableState::Stopped));
}
