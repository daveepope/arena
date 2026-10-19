#[path = "../support/profile_subjects.rs"]
mod profile_subjects;
#[path = "../support/recorded_profiles.rs"]
mod recorded_profiles;

use arena::lifecycle::RunnableState;
use arena_profile::{
    prepare_cpu_profile, CpuProfileError, CpuProfilerBackend, LaunchRequest, PreparedLaunch,
    ShutdownSignal, DEFAULT_CPU_PROFILE_LIMIT_BYTES,
};
use profile_subjects::{await_completed_pass, marker_path, subject_path, PASS_MILLIS};
use recorded_profiles::start_recording;
use std::process::{Command, Stdio};

const INTERPRETER: &str = "python3";

fn busy_loop_request(marker: &std::path::Path) -> LaunchRequest {
    LaunchRequest {
        program: INTERPRETER.into(),
        args: vec![
            subject_path("tools/profile_subjects/busy_loop.py"),
            PASS_MILLIS.to_string(),
            marker.to_string_lossy().into_owned(),
        ],
    }
}

fn spawn_profiled(
    args: &[String],
    env: &[arena_profile::ProfileEnvVar],
    extra: &[(&str, String)],
) -> std::process::Child {
    let mut command = Command::new(INTERPRETER);
    for entry in env {
        command.env(&entry.key, &entry.value);
    }
    for (key, value) in extra {
        command.env(key, value);
    }
    command
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the profiled interpreter")
}

fn terminate(child: &mut std::process::Child) {
    let status = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .expect("send SIGTERM");
    assert!(status.success());
}

#[test]
fn prepare_cpu_profile_python_site_sampler_against_a_busy_loop_reports_folded_stacks() {
    let component_id = "python-site-component-test";
    let recording = start_recording();
    let marker = marker_path("python-site");

    let prepared = prepare_cpu_profile(
        CpuProfilerBackend::PythonSiteSampler,
        busy_loop_request(&marker),
        component_id,
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    )
    .expect("prepare the python site sampler");
    let PreparedLaunch::ArgsAugmented { args, env, shutdown_signal, session } = prepared else {
        panic!("the python site sampler must augment the environment, not wrap the program");
    };
    assert_eq!(shutdown_signal, ShutdownSignal::Terminate);
    let injected = env
        .iter()
        .find(|entry| entry.key == "PYTHONPATH")
        .expect("PYTHONPATH must carry the sampler directory");
    assert!(std::path::Path::new(&injected.value).join("sitecustomize.py").exists());

    let mut child = spawn_profiled(&args, &env, &[]);
    if let Err(reason) = await_completed_pass(&marker, &mut child) {
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_file(&marker);
        panic!("{reason}");
    }
    terminate(&mut child);

    session.finish(&mut child).expect("finish the python site profile");

    let folded = recording
        .recorded
        .folded_for(component_id)
        .expect("folded stacks were not reported");
    assert!(folded.contains("bar"), "the subject's hotspot was not sampled, got: {folded}");
    assert!(
        !folded.contains("sitecustomize.py"),
        "the sampler must not credit its own frames, got: {folded}"
    );
    assert!(recording
        .recorded
        .states_for(component_id)
        .contains(&RunnableState::Stopped));
    assert!(
        !std::path::Path::new(&injected.value).exists(),
        "the sampler directory must be removed once the session finishes"
    );
    let _ = std::fs::remove_file(&marker);
}

#[test]
fn prepare_cpu_profile_python_site_sampler_shadowed_by_another_sitecustomize_faults() {
    let component_id = "python-site-shadowed-component-test";
    let _recording = start_recording();
    let marker = marker_path("python-site-shadowed");
    let shadow = std::env::temp_dir().join(format!("arena-shadow-{}", std::process::id()));
    std::fs::create_dir_all(&shadow).expect("create the shadowing directory");
    std::fs::write(shadow.join("sitecustomize.py"), "pass\n").expect("write the shadow");

    let prepared = prepare_cpu_profile(
        CpuProfilerBackend::PythonSiteSampler,
        busy_loop_request(&marker),
        component_id,
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    )
    .expect("prepare the python site sampler");
    let PreparedLaunch::ArgsAugmented { args, env, session, .. } = prepared else {
        panic!("the python site sampler must augment the environment");
    };
    let ours = env
        .iter()
        .find(|entry| entry.key == "PYTHONPATH")
        .expect("PYTHONPATH")
        .value
        .clone();
    let shadowed = std::env::join_paths([shadow.clone(), std::path::PathBuf::from(&ours)])
        .expect("join the shadow ahead of ours")
        .into_string()
        .expect("utf8");

    let mut child = spawn_profiled(&args, &[], &[("PYTHONPATH", shadowed)]);
    if let Err(reason) = await_completed_pass(&marker, &mut child) {
        let _ = child.kill();
        let _ = child.wait();
        panic!("{reason}");
    }
    terminate(&mut child);

    let outcome = session.finish(&mut child);

    match outcome {
        Err(CpuProfileError::Finish(message)) => assert!(
            message.contains("sitecustomize"),
            "the fault must name the cause, got: {message}"
        ),
        other => panic!("a shadowed sampler must fault, got: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&shadow);
    let _ = std::fs::remove_file(&marker);
}

#[test]
fn prepare_cpu_profile_python_site_sampler_losing_sigprof_faults_rather_than_reporting_a_profile() {
    let component_id = "python-site-sigprof-component-test";
    let _recording = start_recording();
    let marker = marker_path("python-site-sigprof");
    let holder = std::env::temp_dir().join(format!("arena-sigprof-{}.py", std::process::id()));
    let subject = [
        "import glob, os, pathlib, signal, time",
        "signal.signal(signal.SIGPROF, lambda *_: None)",
        "sampler = os.environ['PYTHONPATH'].split(os.pathsep)[-1]",
        "while not glob.glob(os.path.join(sampler, 'status-*.txt')):",
        "    time.sleep(0.01)",
        &format!("pathlib.Path({marker:?}).write_bytes(b'1')"),
        "time.sleep(30)",
        "",
    ]
    .join("\n");
    std::fs::write(&holder, subject).expect("write the subject that takes SIGPROF from the sampler");

    let prepared = prepare_cpu_profile(
        CpuProfilerBackend::PythonSiteSampler,
        LaunchRequest {
            program: INTERPRETER.into(),
            args: vec![holder.to_string_lossy().into_owned()],
        },
        component_id,
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    )
    .expect("prepare the python site sampler");
    let PreparedLaunch::ArgsAugmented { args, env, session, .. } = prepared else {
        panic!("the python site sampler must augment the environment");
    };

    let mut child = spawn_profiled(&args, &env, &[]);
    if let Err(reason) = await_completed_pass(&marker, &mut child) {
        let _ = child.kill();
        let _ = child.wait();
        panic!("{reason}");
    }
    terminate(&mut child);

    let outcome = session.finish(&mut child);

    match outcome {
        Err(CpuProfileError::Finish(message)) => assert!(
            message.contains("replaced SIGPROF"),
            "the refusal must say the signal was taken, got: {message}"
        ),
        other => panic!("a component that takes SIGPROF must fault, got: {other:?}"),
    }
    let _ = std::fs::remove_file(&holder);
    let _ = std::fs::remove_file(&marker);
}
