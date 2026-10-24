#[path = "../support/profile_subjects.rs"]
mod profile_subjects;
#[path = "../support/recorded_profiles.rs"]
mod recorded_profiles;

use arena::lifecycle::RunnableState;
use arena_profile::DEFAULT_CPU_PROFILE_LIMIT_BYTES;
use arena_profile::{prepare_cpu_profile, CpuProfilerBackend, LaunchRequest, PreparedLaunch};
use recorded_profiles::start_recording;
use profile_subjects::{await_completed_pass, marker_path, subject_path, PASS_MILLIS};
use std::io::{BufRead, BufReader, Read};
use std::process::{ChildStderr, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

const SAMPLING_ANNOUNCEMENT: &str = "Sampling process";
const SAMPLING_START_BUDGET: Duration = Duration::from_secs(20);

struct ProfilerOutput {
    stdout: JoinHandle<String>,
    stderr: JoinHandle<String>,
}

impl ProfilerOutput {
    fn report(self) -> String {
        let stdout = self.stdout.join().unwrap_or_default();
        let stderr = self.stderr.join().unwrap_or_default();
        format!("py-spy stdout:\n{stdout}\npy-spy stderr:\n{stderr}")
    }
}

fn watch_for_sampling(stdout: ChildStdout) -> (JoinHandle<String>, mpsc::Receiver<()>) {
    let (announced, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let mut collected = String::new();
        let mut signalled = false;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if !signalled && line.contains(SAMPLING_ANNOUNCEMENT) {
                signalled = announced.send(()).is_ok();
            }
            collected.push_str(&line);
            collected.push('\n');
        }
        collected
    })
    ;
    (handle, rx)
}

fn await_sampling_started(announced: &mpsc::Receiver<()>) -> Result<(), String> {
    announced
        .recv_timeout(SAMPLING_START_BUDGET)
        .map_err(|_| format!("py-spy did not report sampling within {SAMPLING_START_BUDGET:?}"))
}

fn collect_stderr(stderr: ChildStderr) -> JoinHandle<String> {
    std::thread::spawn(move || {
        let mut collected = String::new();
        let mut stderr = stderr;
        let _ = stderr.read_to_string(&mut collected);
        collected
    })
}



#[test]
fn prepare_cpu_profile_real_pyspy_wrapping_python_busy_loop_reports_folded_stacks() {
    let component_id = "pyspy-component-test";
    let recording = start_recording();

    let marker = marker_path("pyspy");
    let request = LaunchRequest {
        program: "python3".into(),
        args: vec![
            subject_path("tools/profile_subjects/busy_loop.py"),
            PASS_MILLIS.to_string(),
            marker.to_string_lossy().into_owned(),
        ],
    };

    let prepared = prepare_cpu_profile(
        CpuProfilerBackend::PySpy,
        request,
        component_id,
        DEFAULT_CPU_PROFILE_LIMIT_BYTES,
    )
    .expect("prepare py-spy profile");
    let PreparedLaunch::Wrapped {
        program,
        args,
        session,
    } = prepared
    else {
        panic!("expected Wrapped variant for PySpy backend");
    };

    let mut wrapping_child = Command::new(&program)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn py-spy wrapping process");

    let stdout = wrapping_child.stdout.take().expect("py-spy stdout pipe");
    let stderr = wrapping_child.stderr.take().expect("py-spy stderr pipe");
    let (stdout_handle, announced) = watch_for_sampling(stdout);
    let output = ProfilerOutput {
        stdout: stdout_handle,
        stderr: collect_stderr(stderr),
    };

    if let Err(reason) = await_sampling_started(&announced)
        .and_then(|()| await_completed_pass(&marker, &mut wrapping_child))
    {
        let _ = wrapping_child.kill();
        let _ = wrapping_child.wait();
        let _ = std::fs::remove_file(&marker);
        panic!("{reason}\n{}", output.report());
    }

    let finished = session.finish(&mut wrapping_child);
    let _ = wrapping_child.wait();
    let _ = std::fs::remove_file(&marker);
    if let Err(error) = finished {
        panic!("finish py-spy profile: {error}\n{}", output.report());
    }

    let folded = recording
        .recorded
        .folded_for(component_id)
        .unwrap_or_else(|| panic!("folded stacks were not reported\n{}", output.report()));
    assert!(
        folded.contains("bar"),
        "the subject's hotspot was not sampled, got: {folded}"
    );
    assert!(recording
        .recorded
        .states_for(component_id)
        .contains(&RunnableState::Stopped));
}
