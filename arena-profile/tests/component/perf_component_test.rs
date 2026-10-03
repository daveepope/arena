use arena_profile::{prepare_cpu_profile, CpuProfilerBackend, LaunchRequest, PreparedLaunch};
use std::process::{Command, Stdio};
use std::time::Duration;

#[test]
fn prepare_cpu_profile_real_perf_wrapping_busy_loop_produces_html_report() {
    let output_path = std::env::temp_dir().join(format!(
        "arena-profile-perf-component-test-{}.html",
        std::process::id()
    ));

    let request = LaunchRequest {
        program: "sh".into(),
        args: vec![
            "-c".to_string(),
            "i=0; while true; do i=$((i+1)); done".to_string(),
        ],
    };

    let prepared =
        prepare_cpu_profile(CpuProfilerBackend::Perf, request, &output_path).expect("prepare perf profile");
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

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
}
