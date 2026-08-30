pub mod async_profiler;
pub mod perf;
pub mod pyspy;

use crate::profiler::CpuProfileError;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn scratch_path(prefix: &str, ext: &str) -> PathBuf {
    let n = SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("{prefix}-{}-{n}.{ext}", std::process::id()))
}

pub fn resolve_binary(
    rlocations: &[&str],
    path_fallback: &'static str,
    binary: &'static str,
    install_hint: &'static str,
) -> Result<String, CpuProfileError> {
    clear_stale_runfiles_manifest_env();
    match runfiles::Runfiles::create() {
        Ok(r) => {
            for rlocation in rlocations {
                if let Some(path) = r.rlocation(rlocation) {
                    if path.exists() {
                        return Ok(path.to_string_lossy().into_owned());
                    }
                }
            }
        }
        Err(e) => {
            tracing::debug!(binary, error = %e, "runfiles resolution unavailable, falling back to PATH");
        }
    }
    if binary_on_path(path_fallback) {
        Ok(path_fallback.to_string())
    } else {
        Err(CpuProfileError::MissingBinary { binary, install_hint })
    }
}

pub fn clear_stale_runfiles_manifest_env() {
    if let Some(manifest) = std::env::var_os("RUNFILES_MANIFEST_FILE") {
        if !manifest.is_empty() && !std::path::Path::new(&manifest).exists() {
            std::env::remove_var("RUNFILES_MANIFEST_FILE");
        }
    }
}

pub fn binary_on_path(binary: &str) -> bool {
    let candidate = std::path::Path::new(binary);
    if candidate.is_absolute() {
        return candidate.exists();
    }
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(binary).is_file()))
        .unwrap_or(false)
}

pub fn map_spawn_error(
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

pub fn signal_interrupt(child: &mut Child) -> std::io::Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }
    let status = Command::new("kill").args(["-INT", &child.id().to_string()]).status()?;
    if !status.success() {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        return Err(std::io::Error::other(format!(
            "kill -INT {} exited with {status}",
            child.id()
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
