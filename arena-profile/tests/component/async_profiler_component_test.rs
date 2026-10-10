use arena_profile::{prepare_cpu_profile, CpuProfilerBackend, LaunchRequest, PreparedLaunch, ShutdownSignal};
use std::process::{Command, Stdio};
use std::time::Duration;

const BUSY_LOOP_JAVA: &str = "import java.nio.file.Files;\nimport java.nio.file.Path;\npublic class BusyLoop {\n    public static void main(String[] args) throws Exception {\n        Files.write(Path.of(args[0]), new byte[] {1});\n        long i = 0;\n        while (true) {\n            i++;\n        }\n    }\n}\n";

const JVM_START_BUDGET: Duration = Duration::from_secs(20);
const SAMPLING_WINDOW: Duration = Duration::from_millis(500);

fn await_jvm_running(marker_path: &std::path::Path, jvm: &mut std::process::Child) {
    let deadline = std::time::Instant::now() + JVM_START_BUDGET;
    while std::time::Instant::now() < deadline {
        if marker_path.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = jvm.kill();
    let _ = jvm.wait();
    panic!("jvm did not reach main() within {JVM_START_BUDGET:?}");
}

#[test]
fn prepare_cpu_profile_real_async_profiler_agent_flag_against_jvm_busy_loop_produces_html_report() {
    let fixture_path = std::env::temp_dir().join(format!(
        "arena-profile-asprof-fixture-{}.java",
        std::process::id()
    ));
    std::fs::write(&fixture_path, BUSY_LOOP_JAVA).expect("write java fixture");

    let output_path = std::env::temp_dir().join(format!(
        "arena-profile-asprof-component-test-{}.html",
        std::process::id()
    ));

    let marker_path = std::env::temp_dir().join(format!(
        "arena-profile-asprof-running-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&marker_path);

    let request = LaunchRequest {
        program: "java".into(),
        args: vec![
            fixture_path.to_string_lossy().into_owned(),
            marker_path.to_string_lossy().into_owned(),
        ],
    };

    let prepared = prepare_cpu_profile(CpuProfilerBackend::AsyncProfiler, request, &output_path)
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

    await_jvm_running(&marker_path, &mut jvm);
    std::thread::sleep(SAMPLING_WINDOW);

    let status = Command::new("kill")
        .args(["-TERM", &jvm.id().to_string()])
        .status()
        .expect("send SIGTERM to jvm");
    assert!(status.success());

    session.finish(&mut jvm).expect("finish async-profiler profile");
    let _ = std::fs::remove_file(&fixture_path);

    let report = std::fs::read_to_string(&output_path).expect("read html report");
    assert!(report.contains("<svg"));
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_file(&marker_path);
}
