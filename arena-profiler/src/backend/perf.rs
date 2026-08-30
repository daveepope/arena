use super::{map_spawn_error, resolve_binary, scratch_path, signal_interrupt, wait_bounded};
use crate::profiler::{CpuProfileError, LaunchRequest};
use crate::sampler::{WrapState, WrappingSampler};
use inferno::collapse::perf::Folder;
use inferno::collapse::Collapse;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const PERF_BINARY: &str = "perf";
const INSTALL_HINT: &str =
    "install the linux-tools package matching your running kernel (e.g. linux-tools-generic)";
const PERF_RLOCATIONS: &[&str] = &["perf"];
const NO_SAMPLES_MARKER: &str = "zero-sized data";
const PARANOID_SYSCTL: &str = "/proc/sys/kernel/perf_event_paranoid";
const PARANOID_MAX_FOR_SAMPLING: i32 = 2;

pub fn no_samples_message() -> String {
    let mut message = String::from(
        "perf recorded no samples, so there is nothing to profile. This almost always means \
         this process is not permitted to read CPU performance counters rather than that the \
         profiled program did no work.",
    );
    match perf_event_paranoid() {
        Some(paranoid) if paranoid > PARANOID_MAX_FOR_SAMPLING => message.push_str(&format!(
            " {PARANOID_SYSCTL} is currently {paranoid}; sampling needs \
             {PARANOID_MAX_FOR_SAMPLING} or lower, or CAP_PERFMON on the perf binary."
        )),
        Some(paranoid) => message.push_str(&format!(
            " {PARANOID_SYSCTL} is currently {paranoid}, which is permissive enough, so the perf \
             binary is most likely missing CAP_PERFMON."
        )),
        None => {}
    }
    message.push_str(
        " Grant the capability once with `sudo setcap cap_perfmon+ep \"$(which perf)\"`, or relax \
         the sysctl with `sudo sysctl -w kernel.perf_event_paranoid=1`. Note that a linux-tools or \
         kernel upgrade replaces the perf binary and drops the capability.",
    );
    message
}

pub fn perf_event_paranoid() -> Option<i32> {
    std::fs::read_to_string(PARANOID_SYSCTL).ok()?.trim().parse().ok()
}

fn folded_is_empty(folded_path: &Path) -> bool {
    std::fs::metadata(folded_path).map(|m| m.len() == 0).unwrap_or(false)
}

pub struct PerfSampler;

impl WrappingSampler for PerfSampler {
    fn wrap(
        &self,
        request: &LaunchRequest,
        _output_path: &Path,
    ) -> Result<(PathBuf, Vec<String>, WrapState), CpuProfileError> {
        let perf = resolve_binary(PERF_RLOCATIONS, PERF_BINARY, PERF_BINARY, INSTALL_HINT)?;
        let data_path = scratch_path("arena-profiler-perf", "data");

        let mut args: Vec<String> = vec![
            "record".into(),
            "-g".into(),
            "-o".into(),
            data_path.to_string_lossy().into_owned(),
            "--".into(),
        ];
        args.push(request.program.to_string_lossy().into_owned());
        args.extend(request.args.iter().cloned());

        Ok((PathBuf::from(perf), args, WrapState::Perf { data_path }))
    }

    fn collect(
        &self,
        state: WrapState,
        wrapping_child: &mut Child,
        budget: Duration,
    ) -> Result<PathBuf, CpuProfileError> {
        let WrapState::Perf { data_path } = state else {
            unreachable!("PerfSampler::collect given a non-perf WrapState");
        };

        signal_interrupt(wrapping_child)
            .map_err(|e| CpuProfileError::Finish(format!("failed to signal perf record: {e}")))?;
        wait_bounded(wrapping_child, budget)
            .map_err(|e| CpuProfileError::Finish(format!("perf record did not exit cleanly: {e}")))?;

        let mut script_child = Command::new(PERF_BINARY)
            .args(["script", "-i"])
            .arg(&data_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| map_spawn_error(PERF_BINARY, INSTALL_HINT, e))?;

        let script_stdout = script_child.stdout.take().expect("stdout was piped");
        let mut script_stderr = script_child.stderr.take().expect("stderr was piped");
        let stderr_thread = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = script_stderr.read_to_end(&mut buf);
            buf
        });

        let folded_path = scratch_path("arena-profiler-perf-folded", "folded");
        let mut folded_file = std::fs::File::create(&folded_path).map_err(|e| {
            CpuProfileError::Finish(format!("failed to create folded stacks file: {e}"))
        })?;
        let collapse_result =
            Folder::default().collapse(BufReader::new(script_stdout), &mut folded_file);

        let script_status = script_child
            .wait()
            .map_err(|e| CpuProfileError::Finish(format!("perf script did not exit cleanly: {e}")))?;
        let script_stderr = stderr_thread.join().unwrap_or_default();
        let _ = std::fs::remove_file(&data_path);

        if !script_status.success() {
            let stderr = String::from_utf8_lossy(&script_stderr);
            if stderr.contains(NO_SAMPLES_MARKER) {
                let _ = std::fs::remove_file(&folded_path);
                return Err(CpuProfileError::Finish(no_samples_message()));
            }
            return Err(CpuProfileError::Finish(format!("perf script failed: {stderr}")));
        }

        collapse_result.map_err(|e| {
            CpuProfileError::Finish(format!("failed to collapse perf script output: {e}"))
        })?;
        folded_file.flush().map_err(|e| {
            CpuProfileError::Finish(format!("failed to flush folded stacks file: {e}"))
        })?;

        if folded_is_empty(&folded_path) {
            let _ = std::fs::remove_file(&folded_path);
            return Err(CpuProfileError::Finish(no_samples_message()));
        }

        Ok(folded_path)
    }
}
