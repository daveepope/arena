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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_path_called_twice_returns_distinct_paths() {
        let first = scratch_path("prefix", "ext");
        let second = scratch_path("prefix", "ext");

        assert_ne!(first, second);
    }

    #[test]
    fn resolve_binary_fallback_on_path_returns_ok() {
        let result = resolve_binary(&[], "sh", "sh", "install sh");

        assert_eq!(result.unwrap(), "sh");
    }

    #[test]
    fn resolve_binary_not_on_path_returns_missing_binary() {
        let result = resolve_binary(
            &[],
            "arena-profile-nonexistent-binary",
            "fake-tool",
            "install fake-tool",
        );

        assert!(matches!(
            result,
            Err(CpuProfileError::MissingBinary { binary: "fake-tool", .. })
        ));
    }

    #[test]
    fn binary_on_path_absolute_existing_path_returns_true() {
        assert!(binary_on_path("/bin/sh"));
    }

    #[test]
    fn binary_on_path_absolute_missing_path_returns_false() {
        assert!(!binary_on_path("/nonexistent/arena-profile-fake-binary"));
    }

    #[test]
    fn map_spawn_error_not_found_returns_missing_binary() {
        let e = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");

        let result = map_spawn_error("perf", "install perf", e);

        assert!(matches!(result, CpuProfileError::MissingBinary { binary: "perf", .. }));
    }

    #[test]
    fn map_spawn_error_other_kind_returns_spawn() {
        let e = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");

        let result = map_spawn_error("perf", "install perf", e);

        assert!(matches!(result, CpuProfileError::Spawn(_)));
    }

    #[test]
    fn signal_interrupt_running_child_returns_ok_and_interrupts() {
        let mut child = Command::new("sleep").arg("5").spawn().expect("spawn sleep");

        let result = signal_interrupt(child.id());

        assert!(result.is_ok());
        std::thread::sleep(Duration::from_millis(100));
        assert!(child.try_wait().unwrap().is_some());
    }

    #[test]
    fn signal_interrupt_already_exited_child_returns_err() {
        let mut child = Command::new("true").spawn().expect("spawn true");
        let _ = child.wait();

        let result = signal_interrupt(child.id());

        assert!(result.is_err());
    }
}
