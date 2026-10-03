use arena_profile::{prepare_cpu_profile, CpuProfilerBackend, LaunchRequest, PreparedLaunch};
use std::process::{Command, Stdio};
use std::time::Duration;

fn resolve_busy_loop_binary() -> String {
    let r = runfiles::Runfiles::create().expect("runfiles available under bazel test");
    r.rlocation("_main/arena-profile/dotnet_busy_loop/net8.0/dotnet_busy_loop.dll.sh")
        .expect("dotnet_busy_loop runfile present")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn prepare_cpu_profile_real_dotnet_trace_wrapping_busy_loop_produces_html_report() {
    let output_path = std::env::temp_dir().join(format!(
        "arena-profile-dotnet-trace-component-test-{}.html",
        std::process::id()
    ));

    let request = LaunchRequest {
        program: resolve_busy_loop_binary().into(),
        args: Vec::new(),
    };

    let prepared = prepare_cpu_profile(CpuProfilerBackend::DotnetTrace, request, &output_path)
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

    std::thread::sleep(Duration::from_millis(1500));

    session.finish(&mut wrapping_child).expect("finish dotnet-trace profile");
    let _ = wrapping_child.wait();

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
}
