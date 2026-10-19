use super::{resolve_binary, sampled_nothing, scratch_path};
use crate::profiler::{CpuProfileError, LaunchRequest};
use crate::augmented::{ArgAugmentingSampler, AugmentState, Augmentation};
use std::path::PathBuf;
use std::time::Duration;

const LIBASYNC_PROFILER_RLOCATIONS: &[&str] = &[
    "_main~_repo_rules~async_profiler_linux_x64/lib/libasyncProfiler.so",
    "_main~_repo_rules~async_profiler_linux_arm64/lib/libasyncProfiler.so",
];
const LIBASYNC_PROFILER_PATH_FALLBACK: &str = "libasyncProfiler.so";
const JAVA_TOOL_OPTIONS: &str = "JAVA_TOOL_OPTIONS";
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
        let folded_path = scratch_path("arena-profile-asprof", "collapsed");

        let folded_pattern = pid_scoped_output(&folded_path, PID_PLACEHOLDER);
        let agent_arg = format!(
            "-agentpath:{}=start,event=cpu,file={},collapsed",
            libasync_profiler,
            folded_pattern.display(),
        );

        Ok(Augmentation {
            args: request.args.clone(),
            env: vec![(JAVA_TOOL_OPTIONS.to_string(), agent_arg)],
            state: AugmentState::AsyncProfiler { folded_path },
        })
    }

    fn collect(
        &self,
        state: AugmentState,
        _budget: Duration,
        profiled_pid: u32,
    ) -> Result<Option<PathBuf>, CpuProfileError> {
        let AugmentState::AsyncProfiler { folded_path } = state;
        let folded_path = pid_scoped_output(&folded_path, &profiled_pid.to_string());
        if !folded_path.exists() {
            return Err(CpuProfileError::Finish(format!(
                "async-profiler output file was not written at {} (the process may have been \
                 force-killed before its shutdown hook could flush the profile)",
                folded_path.display()
            )));
        }
        if sampled_nothing(&folded_path) {
            let _ = std::fs::remove_file(&folded_path);
            return Ok(None);
        }

        Ok(Some(folded_path))
    }
}
