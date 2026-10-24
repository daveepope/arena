pub(crate) mod async_profiler;
pub mod dotnet_trace;
pub mod perf;
#[doc(hidden)]
pub mod python_site;
pub mod pyspy;

use crate::profiler::CpuProfileError;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const SCRATCH_DIR_ATTEMPTS: usize = 8;
#[cfg(unix)]
const SCRATCH_DIR_MODE: u32 = 0o700;
#[cfg(unix)]
const SCRATCH_FILE_MODE: u32 = 0o600;

pub struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    pub fn create(prefix: &str) -> Result<Self, CpuProfileError> {
        let root = std::env::temp_dir();
        let mut last = String::new();
        for _ in 0..SCRATCH_DIR_ATTEMPTS {
            let candidate = root.join(format!(
                "{prefix}-{}-{}",
                std::process::id(),
                random_suffix()
            ));
            match create_private_dir(&candidate) {
                Ok(()) => {
                    let claimed = Self { path: candidate };
                    claimed.refuse_unless_private(&root)?;
                    return Ok(claimed);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    last = format!("{} already exists", candidate.display());
                }
                Err(e) => {
                    return Err(CpuProfileError::Launch(format!(
                        "could not create the private profile directory {}: {e}",
                        candidate.display()
                    )))
                }
            }
        }
        Err(CpuProfileError::Launch(format!(
            "could not create a private profile directory under {} after {SCRATCH_DIR_ATTEMPTS} attempts ({last})",
            root.display()
        )))
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn file_path(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    pub fn create_file(&self, name: &str) -> Result<(PathBuf, std::fs::File), CpuProfileError> {
        let path = self.file_path(name);
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(SCRATCH_FILE_MODE);
        }
        match options.open(&path) {
            Ok(file) => Ok((path, file)),
            Err(e) => Err(CpuProfileError::Launch(format!(
                "could not create {} inside the private profile directory: {e}",
                path.display()
            ))),
        }
    }

    #[cfg(unix)]
    fn refuse_unless_private(&self, root: &std::path::Path) -> Result<(), CpuProfileError> {
        use std::os::unix::fs::MetadataExt;

        let (probe_path, _probe) = self.create_file("owner.probe")?;
        let our_uid = std::fs::metadata(&probe_path)
            .map_err(|e| {
                CpuProfileError::Launch(format!("could not read back the ownership probe: {e}"))
            })?
            .uid();
        let _ = std::fs::remove_file(&probe_path);

        let directory = std::fs::metadata(&self.path).map_err(|e| {
            CpuProfileError::Launch(format!("could not read the private profile directory: {e}"))
        })?;
        if directory.uid() != our_uid {
            return Err(CpuProfileError::Launch(format!(
                "the private profile directory {} is owned by uid {} rather than this process",
                self.path.display(),
                directory.uid()
            )));
        }
        if directory.mode() & 0o777 != SCRATCH_DIR_MODE {
            return Err(CpuProfileError::Launch(format!(
                "the private profile directory {} is mode {:o} rather than {SCRATCH_DIR_MODE:o}",
                self.path.display(),
                directory.mode() & 0o777
            )));
        }

        let root_metadata = std::fs::metadata(root).map_err(|e| {
            CpuProfileError::Launch(format!("could not read {}: {e}", root.display()))
        })?;
        let sticky = root_metadata.mode() & 0o1000 != 0;
        if !sticky && root_metadata.uid() != our_uid {
            return Err(CpuProfileError::Launch(format!(
                "{} is neither owned by this process nor sticky, so a profile directory there could be replaced",
                root.display()
            )));
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn refuse_unless_private(&self, _root: &std::path::Path) -> Result<(), CpuProfileError> {
        Ok(())
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_dir_all(&self.path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    directory = %self.path.display(),
                    error = %e,
                    phase = "profile_scratch_teardown",
                    "the private profile directory could not be removed"
                );
            }
        }
    }
}

fn random_suffix() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 9];
    rand::thread_rng().fill(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn create_private_dir(path: &std::path::Path) -> std::io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(SCRATCH_DIR_MODE);
    }
    builder.create(path)
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

pub fn sampled_nothing(profile_path: &std::path::Path) -> bool {
    std::fs::metadata(profile_path)
        .map(|metadata| metadata.len() == 0)
        .unwrap_or(false)
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
