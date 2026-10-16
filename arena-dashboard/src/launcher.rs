use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;
use std::time::{Duration, Instant};

pub const DEFAULT_PORT: u16 = 4873;

const EXECUTABLE_NAMES: [&str; 3] = [
    "arena-dashboard-server-linux",
    "arena-dashboard-server-macos",
    "arena-dashboard-server.exe",
];

const RUNFILES_BASES: [&str; 3] = ["_main", "arena", ""];

#[derive(Debug)]
pub enum LaunchError {
    ExecutableNotFound,
    Spawn(io::Error),
    Timeout { url: String, waited: Duration },
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaunchError::ExecutableNotFound => {
                write!(f, "arena-dashboard server executable not found")
            }
            LaunchError::Spawn(e) => write!(f, "failed to spawn arena-dashboard server: {e}"),
            LaunchError::Timeout { url, waited } => write!(
                f,
                "arena-dashboard server at {url} did not become ready within {waited:?}"
            ),
        }
    }
}

impl std::error::Error for LaunchError {}

fn is_arena_dashboard_response(response: &reqwest::blocking::Response) -> bool {
    response.status().is_success()
        && response
            .headers()
            .get(reqwest::header::CACHE_CONTROL)
            .is_some_and(|value| value == "no-store")
}

pub fn poll_ready(url: &str, timeout: Duration) -> bool {
    let client = reqwest::blocking::Client::new();
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(response) = client.get(url).timeout(Duration::from_millis(500)).send() {
            if is_arena_dashboard_response(&response) {
                return true;
            }
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

static CLEAR_STALE_RUNFILES_MANIFEST_ENV_ONCE: Once = Once::new();

fn clear_stale_runfiles_manifest_env() {
    CLEAR_STALE_RUNFILES_MANIFEST_ENV_ONCE.call_once(|| {
        if let Some(manifest) = std::env::var_os("RUNFILES_MANIFEST_FILE") {
            if !manifest.is_empty() && !Path::new(&manifest).exists() {
                std::env::remove_var("RUNFILES_MANIFEST_FILE");
            }
        }
    });
}

pub fn find_executable(search_dir: Option<&Path>) -> Option<PathBuf> {
    if let Some(dir) = search_dir {
        for name in EXECUTABLE_NAMES {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    clear_stale_runfiles_manifest_env();
    let runfiles = runfiles::Runfiles::create().ok()?;
    for base in RUNFILES_BASES {
        let rel = if base.is_empty() {
            "arena-dashboard/server".to_string()
        } else {
            format!("{base}/arena-dashboard/server")
        };
        if let Some(path) = runfiles.rlocation(rel) {
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

pub fn resolve_port(port: u16) -> u16 {
    if port == 0 {
        DEFAULT_PORT
    } else {
        port
    }
}

pub fn ensure_running(port: u16, search_dir: Option<&Path>) -> Result<String, LaunchError> {
    let port = resolve_port(port);
    let url = format!("http://127.0.0.1:{port}");

    if poll_ready(&url, Duration::from_millis(300)) {
        tracing::info!(url = %url, "arena-dashboard already running");
        return Ok(url);
    }

    tracing::debug!(url = %url, phase = "launch_begin", "starting arena-dashboard server");
    let sw = Instant::now();

    let exe = find_executable(search_dir).ok_or(LaunchError::ExecutableNotFound)?;
    tracing::debug!(exe = %exe.display(), "resolved arena-dashboard server executable");

    let mut command = Command::new(exe);
    command
        .arg(port.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command.spawn().map_err(LaunchError::Spawn)?;

    let timeout = Duration::from_secs(10);
    if poll_ready(&url, timeout) {
        tracing::info!(url = %url, elapsed = ?sw.elapsed(), "arena-dashboard server started");
        Ok(url)
    } else {
        tracing::warn!(url = %url, waited = ?timeout, "arena-dashboard server did not become ready");
        Err(LaunchError::Timeout {
            url,
            waited: timeout,
        })
    }
}
