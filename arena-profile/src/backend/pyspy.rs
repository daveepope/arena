use super::{resolve_binary, scratch_path, signal_interrupt, wait_bounded};
use crate::profiler::{CpuProfileError, LaunchRequest};
use crate::sampler::{WrapState, WrappingSampler};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

const PYSPY_RLOCATIONS: &[&str] = &["rules_rust~~crate~profiling_tools__py-spy-0.4.2/py-spy__bin"];
const PYSPY_PATH_FALLBACK: &str = "py-spy";
const INSTALL_HINT: &str = "install py-spy (e.g. `pip install py-spy`) and ensure it is on PATH";

pub(crate) struct PySpySampler;

impl WrappingSampler for PySpySampler {
    fn wrap(
        &self,
        request: &LaunchRequest,
        _output_path: &Path,
    ) -> Result<(PathBuf, Vec<String>, WrapState), CpuProfileError> {
        let py_spy = resolve_binary(PYSPY_RLOCATIONS, PYSPY_PATH_FALLBACK, PYSPY_PATH_FALLBACK, INSTALL_HINT)?;
        let folded_path = scratch_path("arena-profile-pyspy", "folded");

        let mut args: Vec<String> = vec![
            "record".into(),
            "-o".into(),
            folded_path.to_string_lossy().into_owned(),
            "--format".into(),
            "raw".into(),
            "--".into(),
        ];
        args.push(request.program.to_string_lossy().into_owned());
        args.extend(request.args.iter().cloned());

        Ok((PathBuf::from(py_spy), args, WrapState::PySpy { folded_path }))
    }

    fn collect(
        &self,
        state: WrapState,
        wrapping_child: &mut Child,
        budget: Duration,
    ) -> Result<PathBuf, CpuProfileError> {
        let WrapState::PySpy { folded_path } = state else {
            unreachable!("PySpySampler::collect given a non-py-spy WrapState");
        };

        signal_interrupt(wrapping_child)
            .map_err(|e| CpuProfileError::Finish(format!("failed to signal py-spy record: {e}")))?;
        let status = wait_bounded(wrapping_child, budget).map_err(|e| {
            CpuProfileError::Finish(format!("py-spy record did not exit cleanly: {e}"))
        })?;

        if !status.success() {
            return Err(CpuProfileError::Finish(format!(
                "py-spy record exited with {status}"
            )));
        }

        Ok(folded_path)
    }
}
