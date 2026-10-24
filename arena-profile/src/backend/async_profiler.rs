use super::{resolve_binary, sampled_nothing, ScratchDir};
use crate::profiler::{CpuProfileError, LaunchRequest, ProfileEnvVar};
use crate::augmented::{ArgAugmentingSampler, AugmentState, Augmentation};
use std::path::PathBuf;
use std::time::Duration;

const LIBASYNC_PROFILER_RLOCATIONS: &[&str] = &[
    "_main~_repo_rules~async_profiler_linux_x64/lib/libasyncProfiler.so",
    "_main~_repo_rules~async_profiler_linux_arm64/lib/libasyncProfiler.so",
    "_main~_repo_rules~async_profiler_macos/lib/libasyncProfiler.dylib",
];
#[cfg(target_os = "macos")]
const LIBASYNC_PROFILER_PATH_FALLBACK: &str = "libasyncProfiler.dylib";
#[cfg(not(target_os = "macos"))]
const LIBASYNC_PROFILER_PATH_FALLBACK: &str = "libasyncProfiler.so";
const JAVA_TOOL_OPTIONS: &str = "JAVA_TOOL_OPTIONS";
const NO_SAMPLES_REASON: &str = "async-profiler recorded no samples for this component; the \
component may have used no cpu while it was profiled, or the jvm never loaded the agent handed to \
it in JAVA_TOOL_OPTIONS";
#[cfg(target_os = "macos")]
const INSTALL_HINT: &str = "install async-profiler and ensure libasyncProfiler.dylib is on PATH";
#[cfg(not(target_os = "macos"))]
const INSTALL_HINT: &str = "install async-profiler and ensure libasyncProfiler.so is on PATH";

const PID_PLACEHOLDER: &str = "%p";

pub(crate) fn pid_scoped_output(folded_path: &std::path::Path, pid: &str) -> PathBuf {
    folded_path.with_extension(format!("{pid}.collapsed"))
}

pub(crate) struct AsyncProfilerSampler;

impl ArgAugmentingSampler for AsyncProfilerSampler {
    fn augment(&self, request: &LaunchRequest) -> Result<Augmentation, CpuProfileError> {
        let libasync_profiler = resolve_binary(
            LIBASYNC_PROFILER_RLOCATIONS,
            LIBASYNC_PROFILER_PATH_FALLBACK,
            LIBASYNC_PROFILER_PATH_FALLBACK,
            INSTALL_HINT,
        )?;
        let scratch = ScratchDir::create("arena-profile-asprof")?;
        let folded_path = scratch.file_path("profile.collapsed");

        let folded_pattern = pid_scoped_output(&folded_path, PID_PLACEHOLDER);
        let agent_arg = format!(
            "-agentpath:{}=start,event=cpu,file={},collapsed",
            libasync_profiler,
            folded_pattern.display(),
        );

        Ok(Augmentation {
            args: request.args.clone(),
            env: vec![ProfileEnvVar::space_joined(JAVA_TOOL_OPTIONS, agent_arg)],
            state: AugmentState::AsyncProfiler { scratch, folded_path },
        })
    }

    fn collect(
        &self,
        state: AugmentState,
        _budget: Duration,
        profiled_pid: u32,
        limit_bytes: usize,
    ) -> Result<String, CpuProfileError> {
        let AugmentState::AsyncProfiler { scratch: _scratch, folded_path } = state else {
            return Err(CpuProfileError::Finish(
                "async-profiler was handed another backend's state".to_string(),
            ));
        };
        let folded_path = pid_scoped_output(&folded_path, &profiled_pid.to_string());
        if !folded_path.exists() {
            return Err(CpuProfileError::Finish(format!(
                "async-profiler output file was not written at {} (the process may have been \
                 force-killed before its shutdown hook could flush the profile)",
                folded_path.display()
            )));
        }
        if sampled_nothing(&folded_path) {
            return Err(CpuProfileError::Finish(
                NO_SAMPLES_REASON.to_string(),
            ));
        }

        crate::profiler::read_collected(&folded_path, limit_bytes)
    }
}
