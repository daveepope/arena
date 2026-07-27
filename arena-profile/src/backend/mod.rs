pub(crate) mod async_profiler;
pub(crate) mod perf;
pub(crate) mod pyspy;

use crate::profiler::CpuProfileError;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) fn scratch_path(prefix: &str, ext: &str) -> PathBuf {
    let n = SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("{prefix}-{}-{n}.{ext}", std::process::id()))
}

pub(crate) fn resolve_binary(
    rlocations: &[&str],
    path_fallback: &'static str,
    binary: &'static str,
    install_hint: &'static str,
) -> Result<String, CpuProfileError> {
    if let Ok(r) = runfiles::Runfiles::create() {
        for rlocation in rlocations {
            if let Some(path) = r.rlocation(rlocation) {
                if path.exists() {
                    return Ok(path.to_string_lossy().into_owned());
                }
            }
        }
    }
    if binary_on_path(path_fallback) {
        Ok(path_fallback.to_string())
    } else {
        Err(CpuProfileError::MissingBinary { binary, install_hint })
    }
}

fn binary_on_path(binary: &str) -> bool {
    let candidate = std::path::Path::new(binary);
    if candidate.is_absolute() {
        return candidate.exists();
    }
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(binary).is_file()))
        .unwrap_or(false)
}

pub(crate) fn map_spawn_error(
    binary: &'static str,
    install_hint: &'static str,
    e: std::io::Error,
) -> CpuProfileError {
    if e.kind() == std::io::ErrorKind::NotFound {
        CpuProfileError::MissingBinary { binary, install_hint }
    } else {
        CpuProfileError::Spawn(e)
    }
}

pub(crate) fn signal_interrupt(pid: u32) -> std::io::Result<()> {
    let status = Command::new("kill").args(["-INT", &pid.to_string()]).status()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "kill -INT {pid} exited with {status}"
        )));
    }
    Ok(())
}

pub fn wait_bounded(
    child: &mut Child,
    budget: Duration,
) -> std::io::Result<std::process::ExitStatus> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if start.elapsed() >= budget {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "recorder process did not exit within the shutdown budget",
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
