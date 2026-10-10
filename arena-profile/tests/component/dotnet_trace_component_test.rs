use arena_profile::{prepare_cpu_profile, CpuProfilerBackend, LaunchRequest, PreparedLaunch};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const DOTNET_START_BUDGET: Duration = Duration::from_secs(20);
const SAMPLING_WINDOW: Duration = Duration::from_millis(500);

fn resolve_busy_loop_binary() -> String {
    let r = runfiles::Runfiles::create().expect("runfiles available under bazel test");
    r.rlocation("_main/arena-profile/dotnet_busy_loop/net8.0/dotnet_busy_loop.dll.sh")
        .expect("dotnet_busy_loop runfile present")
        .to_string_lossy()
        .into_owned()
}

fn await_busy_loop_running(marker_path: &std::path::Path, wrapping_child: &mut Child) {
    let deadline = std::time::Instant::now() + DOTNET_START_BUDGET;
    while std::time::Instant::now() < deadline {
        if marker_path.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = wrapping_child.kill();
    let _ = wrapping_child.wait();
    panic!("dotnet busy loop did not reach its marker write within {DOTNET_START_BUDGET:?}");
}

#[test]
fn prepare_cpu_profile_real_dotnet_trace_wrapping_busy_loop_produces_html_report() {
    let output_path = std::env::temp_dir().join(format!(
        "arena-profile-dotnet-trace-component-test-{}.html",
        std::process::id()
    ));
    let marker_path = std::env::temp_dir().join(format!(
        "arena-profile-dotnet-trace-running-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&marker_path);

    let request = LaunchRequest {
        program: resolve_busy_loop_binary().into(),
        args: vec![marker_path.to_string_lossy().into_owned()],
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

    await_busy_loop_running(&marker_path, &mut wrapping_child);
    std::thread::sleep(SAMPLING_WINDOW);

    session.finish(&mut wrapping_child).expect("finish dotnet-trace profile");
    let _ = wrapping_child.wait();

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_file(&marker_path);
}
