use super::ScratchDir;
use crate::augmented::{ArgAugmentingSampler, AugmentState, Augmentation};
use crate::profiler::{read_collected, CpuProfileError, LaunchRequest, ProfileEnvVar};
use std::time::Duration;

const PYTHONPATH: &str = "PYTHONPATH";
const SITE_FILE_NAME: &str = "sitecustomize.py";
const SAMPLE_INTERVAL_SECONDS: f64 = 0.01;
const FIRST_FLUSH_SECONDS: f64 = 0.1;
const FLUSH_CEILING_SECONDS: f64 = 1.0;
const BYTES_PER_STACK_BUDGET: usize = 256;
const PAYLOAD: &str = include_str!("python_site_payload.py");

pub fn folded_name(pid: u32) -> String {
    format!("profile-{pid}.folded")
}


pub fn status_name(pid: u32) -> String {
    format!("status-{pid}.txt")
}

pub fn payload_source(limit_bytes: usize) -> String {
    let source = PAYLOAD
        .replace("__ARENA_INTERVAL_SECONDS__", &format!("{SAMPLE_INTERVAL_SECONDS}"))
        .replace("__ARENA_FIRST_FLUSH_SECONDS__", &format!("{FIRST_FLUSH_SECONDS}"))
        .replace("__ARENA_FLUSH_CEILING_SECONDS__", &format!("{FLUSH_CEILING_SECONDS}"))
        .replace("__ARENA_MAX_DISTINCT_STACKS__", &(limit_bytes / BYTES_PER_STACK_BUDGET).to_string())
        .replace("__ARENA_MAX_BYTES__", &limit_bytes.to_string());
    debug_assert!(
        !source.contains("__ARENA_"),
        "every placeholder in the python sampler must be filled before it is written"
    );
    source
}

pub fn refusal_from_status(status: &str) -> Option<String> {
    let first = status.lines().next().unwrap_or_default().trim();
    match first.strip_prefix("refused ") {
        Some(reason) if !reason.is_empty() => Some(reason.to_string()),
        _ => None,
    }
}

pub struct PythonSiteSampler {
    limit_bytes: usize,
}

impl PythonSiteSampler {
    pub fn new(limit_bytes: usize) -> Self {
        Self { limit_bytes }
    }
}

impl ArgAugmentingSampler for PythonSiteSampler {
    fn augment(&self, request: &LaunchRequest) -> Result<Augmentation, CpuProfileError> {
        let scratch = ScratchDir::create("arena-profile-pysite")?;
        let (_path, mut file) = scratch.create_file(SITE_FILE_NAME)?;
        std::io::Write::write_all(&mut file, payload_source(self.limit_bytes).as_bytes()).map_err(
            |e| CpuProfileError::Launch(format!("could not write the python sampler: {e}")),
        )?;

        Ok(Augmentation {
            args: request.args.clone(),
            env: vec![ProfileEnvVar::path_joined(
                PYTHONPATH,
                scratch.path().to_string_lossy().into_owned(),
            )],
            state: AugmentState::PythonSite { scratch },
        })
    }

    fn collect(
        &self,
        state: AugmentState,
        _budget: Duration,
        profiled_pid: u32,
        limit_bytes: usize,
    ) -> Result<Option<String>, CpuProfileError> {
        let AugmentState::PythonSite { scratch } = state else {
            return Err(CpuProfileError::Finish(
                "the python sampler was handed another backend's state".to_string(),
            ));
        };

        let status_path = scratch.file_path(&status_name(profiled_pid));
        if let Ok(status) = std::fs::read_to_string(&status_path) {
            return Err(CpuProfileError::Finish(match refusal_from_status(&status) {
                Some(reason) => format!("the python sampler refused to profile: {reason}"),
                None => format!(
                    "the python sampler refused to profile and was stopped before it could say \
                     why, leaving an empty status at {}",
                    status_path.display()
                ),
            }));
        }

        let folded_path = scratch.file_path(&folded_name(profiled_pid));
        if !folded_path.exists() {
            return Err(CpuProfileError::Finish(format!(
                "the python sampler wrote no profile at {}, which happens when the component was \
                 terminated before the sampler flushed, when another sitecustomize module took \
                 precedence on PYTHONPATH, or when the interpreter is older than 3.11",
                folded_path.display()
            )));
        }
        read_collected(&folded_path, limit_bytes).map(Some)
    }
}
