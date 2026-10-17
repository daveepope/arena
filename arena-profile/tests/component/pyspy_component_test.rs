#[path = "../support/recorded_profiles.rs"]
mod recorded_profiles;

use arena_profile::DEFAULT_CPU_PROFILE_LIMIT_BYTES;
use arena_profile::{prepare_cpu_profile, CpuProfilerBackend, LaunchRequest, PreparedLaunch};
use arena::lifecycle::RunnableState;
use recorded_profiles::start_recording;
use std::process::{Command, Stdio};
use std::time::Duration;

#[test]
fn prepare_cpu_profile_real_pyspy_wrapping_python_busy_loop_reports_folded_stacks() {
    let component_id = "pyspy-component-test";
    let recording = start_recording();

    let request = LaunchRequest {
        program: "python3".into(),
        args: vec!["-c".to_string(), "while True:\n    pass\n".to_string()],
    };

    let prepared = prepare_cpu_profile(CpuProfilerBackend::PySpy, request, component_id, DEFAULT_CPU_PROFILE_LIMIT_BYTES)
        .expect("prepare py-spy profile");
    let PreparedLaunch::Wrapped { program, args, session } = prepared else {
        panic!("expected Wrapped variant for PySpy backend");
    };

    let mut wrapping_child = Command::new(&program)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn py-spy wrapping process");

    std::thread::sleep(Duration::from_millis(500));

    session.finish(&mut wrapping_child).expect("finish py-spy profile");
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
