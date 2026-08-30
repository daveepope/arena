use arena_profiler::{
    prepare_cpu_profile, CpuProfilerBackend, LaunchRequest, PreparedLaunch, ShutdownSignal,
};
use std::process::{Command, Stdio};
use std::time::Duration;

const BUSY_LOOP_JAVA: &str = "public class BusyLoop {\n    public static void main(String[] args) {\n        long i = 0;\n        while (true) {\n            i++;\n        }\n    }\n}\n";

fn java_binary() -> std::path::PathBuf {
    let runtime = std::env::var("ARENA_JAVA_RUNTIME").expect("ARENA_JAVA_RUNTIME is set by BUILD");
    let relative = std::path::Path::new(&runtime).join("bin/java");
    if relative.exists() {
        return relative;
    }
    std::env::current_dir()
        .expect("read current directory")
        .join(&runtime)
        .join("bin/java")
}

#[test]
fn prepare_cpu_profile_real_async_profiler_agent_flag_against_jvm_busy_loop_produces_html_report() {
    let java = java_binary();
    let fixture_path = std::env::temp_dir()
        .join(format!("arena-profiler-asprof-fixture-{}.java", std::process::id()));
    std::fs::write(&fixture_path, BUSY_LOOP_JAVA).expect("write java fixture");

    let output_path = std::env::temp_dir().join(format!(
        "arena-profiler-asprof-component-test-{}.html",
        std::process::id()
    ));

    let request = LaunchRequest {
        program: java.clone(),
        args: vec![fixture_path.to_string_lossy().into_owned()],
    };

    let prepared = prepare_cpu_profile(CpuProfilerBackend::AsyncProfiler, request, &output_path)
        .expect("prepare async-profiler profile");
    let PreparedLaunch::Augmented { args, env_vars, shutdown_signal, session } = prepared else {
        panic!("expected Augmented variant for AsyncProfiler backend");
    };
    assert_eq!(shutdown_signal, ShutdownSignal::Terminate);

    let mut jvm = Command::new(&java)
        .envs(env_vars.iter().map(|(k, v)| (k.clone(), v.clone())))
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jvm with agent flag");

    std::thread::sleep(Duration::from_millis(500));

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
}
