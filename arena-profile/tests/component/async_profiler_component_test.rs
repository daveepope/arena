#[path = "../support/recorded_profiles.rs"]
mod recorded_profiles;

use arena_profile::DEFAULT_CPU_PROFILE_LIMIT_BYTES;
use arena_profile::{prepare_cpu_profile, CpuProfilerBackend, LaunchRequest, PreparedLaunch, ShutdownSignal};
use arena::lifecycle::RunnableState;
use recorded_profiles::start_recording;
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
fn prepare_cpu_profile_real_async_profiler_agent_flag_against_jvm_busy_loop_reports_folded_stacks() {
    let fixture_path = std::env::temp_dir().join(format!(
        "arena-profile-asprof-fixture-{}.java",
        std::process::id()
    ));
    std::fs::write(&fixture_path, BUSY_LOOP_JAVA).expect("write java fixture");

    let component_id = "async_profiler-component-test";
    let recording = start_recording();

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

    await_jvm_running(&marker_path, &mut jvm);
    std::thread::sleep(SAMPLING_WINDOW);

    let status = Command::new("kill")
        .args(["-TERM", &jvm.id().to_string()])
        .status()
        .expect("send SIGTERM to jvm");
    assert!(status.success());

    session.finish(&mut jvm).expect("finish async-profiler profile");
    let _ = std::fs::remove_file(&fixture_path);

    let folded = recording
        .recorded
        .folded_for(component_id)
        .expect("folded stacks were not reported");
    assert!(!folded.trim().is_empty(), "folded stacks were empty");
    assert!(recording
        .recorded
        .states_for(component_id)
        .contains(&RunnableState::Stopped));
    let _ = std::fs::remove_file(&marker_path);
}
