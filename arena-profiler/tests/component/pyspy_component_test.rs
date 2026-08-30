use arena_profiler::{prepare_cpu_profile, CpuProfilerBackend, LaunchRequest, PreparedLaunch};
use std::process::{Command, Stdio};
use std::time::Duration;

#[test]
fn prepare_cpu_profile_real_pyspy_wrapping_python_busy_loop_produces_html_report() {
    let output_path = std::env::temp_dir().join(format!(
        "arena-profiler-pyspy-component-test-{}.html",
        std::process::id()
    ));

    let request = LaunchRequest {
        program: "python3".into(),
        args: vec!["-c".to_string(), "while True:\n    pass\n".to_string()],
    };

    let prepared = prepare_cpu_profile(CpuProfilerBackend::PySpy, request, &output_path)
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

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
}
