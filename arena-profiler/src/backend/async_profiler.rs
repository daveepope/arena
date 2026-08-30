use super::{resolve_binary, scratch_path};
use crate::profiler::{CpuProfileError, LaunchRequest};
use crate::sampler::{ArgAugmentingSampler, AugmentState};
use std::path::{Path, PathBuf};
use std::time::Duration;

const LIBASYNC_PROFILER_RLOCATIONS: &[&str] = &[
    "_main~_repo_rules~async_profiler_linux_x64/lib/libasyncProfiler.so",
    "_main~_repo_rules~async_profiler_linux_arm64/lib/libasyncProfiler.so",
];
const LIBASYNC_PROFILER_PATH_FALLBACK: &str = "libasyncProfiler.so";
const JVM_TOOL_OPTIONS_ENV_VAR: &str = "JAVA_TOOL_OPTIONS";
const INSTALL_HINT: &str = "install async-profiler and ensure libasyncProfiler.so is on PATH";

pub struct AsyncProfilerSampler;

impl ArgAugmentingSampler for AsyncProfilerSampler {
    fn augment(
        &self,
        request: &LaunchRequest,
        _output_path: &Path,
    ) -> Result<(Vec<String>, Vec<(String, String)>, AugmentState), CpuProfileError> {
        let libasync_profiler = resolve_binary(
            LIBASYNC_PROFILER_RLOCATIONS,
            LIBASYNC_PROFILER_PATH_FALLBACK,
            LIBASYNC_PROFILER_PATH_FALLBACK,
            INSTALL_HINT,
        )?;
        let folded_path = scratch_path("arena-profiler-asprof", "collapsed");

        let agent_option = format!(
            "-agentpath:{}=start,event=cpu,file={},collapsed",
            libasync_profiler,
            folded_path.display(),
        );

        Ok((
            request.args.clone(),
            vec![(JVM_TOOL_OPTIONS_ENV_VAR.to_string(), agent_option)],
            AugmentState::AsyncProfiler { folded_path },
        ))
    }

    fn collect(&self, state: AugmentState, _budget: Duration) -> Result<PathBuf, CpuProfileError> {
        let AugmentState::AsyncProfiler { folded_path } = state;
        if !folded_path.exists() {
            return Err(CpuProfileError::Finish(format!(
                "async-profiler output file was not written at {} (the process may have been \
                 force-killed before its shutdown hook could flush the profile)",
                folded_path.display()
            )));
        }
        Ok(folded_path)
    }
}
