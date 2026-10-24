use super::{
    resolve_binary, sampled_nothing, signal_interrupt, wait_bounded, ScratchDir,
};
use crate::profiler::{CpuProfileError, LaunchRequest};
use crate::wrapped::{WrapState, WrappingSampler};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

const PYSPY_RLOCATIONS: &[&str] = &["rules_rust~~crate~profiling_tools__py-spy-0.4.2/py-spy__bin"];
const PYSPY_PATH_FALLBACK: &str = "py-spy";
const INSTALL_HINT: &str = "install py-spy (e.g. `pip install py-spy`) and ensure it is on PATH";

pub struct PySpySampler;

pub fn record_args(folded_path: &Path, request: &LaunchRequest) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "record".into(),
        "-o".into(),
        folded_path.to_string_lossy().into_owned(),
        "--format".into(),
        "raw".into(),
        "--idle".into(),
        "--".into(),
    ];
    args.push(request.program.to_string_lossy().into_owned());
    args.extend(request.args.iter().cloned());
    args
}

impl WrappingSampler for PySpySampler {
    fn wrap(
        &self,
        request: &LaunchRequest,
    ) -> Result<(PathBuf, Vec<String>, WrapState), CpuProfileError> {
        let py_spy = resolve_binary(PYSPY_RLOCATIONS, PYSPY_PATH_FALLBACK, PYSPY_PATH_FALLBACK, INSTALL_HINT)?;
        let scratch = ScratchDir::create("arena-profile-pyspy")?;
        let folded_path = scratch.file_path("profile.folded");
        let args = record_args(&folded_path, request);

        Ok((PathBuf::from(py_spy), args, WrapState::PySpy { scratch, folded_path }))
    }

    fn collect(
        &self,
        state: WrapState,
        wrapping_child: &mut Child,
        budget: Duration,
        limit_bytes: usize,
    ) -> Result<String, CpuProfileError> {
        let WrapState::PySpy { scratch: _scratch, folded_path } = state else {
            unreachable!("PySpySampler::collect given a non-py-spy WrapState");
        };

        signal_interrupt(wrapping_child)
            .map_err(|e| CpuProfileError::Finish(format!("failed to signal py-spy record: {e}")))?;
        let status = wait_bounded(wrapping_child, budget).map_err(|e| {
            CpuProfileError::Finish(format!("py-spy record did not exit cleanly: {e}"))
        })?;

        // py-spy's own exit status reflects how the SIGINT we send to stop sampling was
        // delivered (often a raw signal termination rather than a caught exit(0)), not
        // whether sampling itself succeeded, so success is judged by whether it actually
        // wrote the folded stacks file.
        if !folded_path.exists() {
            return Err(CpuProfileError::Finish(format!(
                "py-spy wrote no folded stacks at {} and exited with {status}; it attaches to the \
                 process it launches, so a component whose command re-executes into another \
                 process or exits before sampling starts leaves nothing to record",
                folded_path.display()
            )));
        }
        if sampled_nothing(&folded_path) {
            return Err(CpuProfileError::Finish(format!(
                "py-spy recorded no samples for this component and exited with {status}; the \
                 component may have used no cpu while it was profiled, or its python frames were \
                 never visible to py-spy",
            )));
        }

        crate::profiler::read_collected(&folded_path, limit_bytes)
    }
}
