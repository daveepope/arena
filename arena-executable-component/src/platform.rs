use std::path::PathBuf;

// Windows executables need a `.exe` suffix, but callers commonly build
// extension-less paths (matching how the same source builds on Unix); if the
// bare path doesn't exist, try the platform's actual executable name before
// giving up, so callers don't need to special-case this themselves.
#[cfg(windows)]
pub fn resolve_executable_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_none() && !path.exists() {
        let with_extension = path.with_extension(std::env::consts::EXE_EXTENSION);
        if with_extension.exists() {
            return with_extension;
        }
    }
    path
}

#[cfg(not(windows))]
pub fn resolve_executable_extension(path: PathBuf) -> PathBuf {
    path
}
