use std::path::{Path, PathBuf};

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

// A bare command name (no path separator, e.g. "powershell" or "sh") names a
// PATH-resolved command, not a path relative to the component's working
// directory; resolving it against `current_dir`'s ancestors would rewrite it
// into a path that can never exist and the command would never spawn.
fn is_bare_command_name(path: &Path) -> bool {
    path.parent().is_some_and(|parent| parent.as_os_str().is_empty())
}

pub fn resolve_configured_executable_path(path: PathBuf, current_dir: &Path) -> PathBuf {
    if is_bare_command_name(&path) {
        return path;
    }

    let resolved = if path.is_absolute() {
        path
    } else {
        current_dir
            .ancestors()
            .find_map(|ancestor| {
                let candidate = ancestor.join(&path);
                if candidate.exists() {
                    Some(candidate)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| current_dir.join(&path))
    };

    resolve_executable_extension(resolved)
}
