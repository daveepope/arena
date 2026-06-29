use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::{Duration, SystemTime};

use notify::{RecursiveMode, Watcher};

const POLL_INTERVAL: Duration = Duration::from_millis(300);

pub fn transcripts_root() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let workspace = std::env::current_dir().ok()?;
    Some(session_root(Path::new(&home), &workspace))
}

pub fn session_root(home: &Path, workspace: &Path) -> PathBuf {
    home.join(".cursor")
        .join("projects")
        .join(workspace_slug(workspace))
        .join("agent-transcripts")
}

fn workspace_slug(workspace: &Path) -> String {
    workspace
        .to_string_lossy()
        .trim_start_matches('/')
        .replace('/', "-")
}

pub fn scan_sessions(root: &Path) -> Vec<PathBuf> {
    let mut sessions = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return sessions;
    };
    for entry in entries.flatten() {
        let directory = entry.path();
        if !directory.is_dir() {
            continue;
        }
        let Ok(inner) = fs::read_dir(&directory) else {
            continue;
        };
        for file in inner.flatten() {
            let path = file.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
                sessions.push(path);
            }
        }
    }
    sessions
}

pub fn find_new_session(root: &Path, known: &HashSet<PathBuf>) -> Option<PathBuf> {
    scan_sessions(root)
        .into_iter()
        .find(|path| !known.contains(path))
}

pub fn newest_session(mut dated: Vec<(SystemTime, PathBuf)>) -> Option<PathBuf> {
    dated.sort_by_key(|(modified, _)| *modified);
    dated.pop().map(|(_, path)| path)
}

pub fn latest_session() -> io::Result<PathBuf> {
    let root = transcripts_root().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not resolve Cursor session directory for this workspace",
        )
    })?;
    latest_session_in(&root)
}

pub fn latest_session_in(root: &Path) -> io::Result<PathBuf> {
    let dated: Vec<(SystemTime, PathBuf)> = scan_sessions(root)
        .into_iter()
        .filter_map(|path| {
            let modified = fs::metadata(&path).ok()?.modified().ok()?;
            Some((modified, path))
        })
        .collect();

    newest_session(dated).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("no sessions found in {}", root.display()),
        )
    })
}

pub fn wait_for_new_session() -> io::Result<PathBuf> {
    let root = transcripts_root().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not resolve Cursor session directory for this workspace",
        )
    })?;
    wait_for_new_session_in(&root)
}

pub fn wait_for_new_session_in(root: &Path) -> io::Result<PathBuf> {
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Cursor session directory not found: {}", root.display()),
        ));
    }

    let known: HashSet<PathBuf> = scan_sessions(root).into_iter().collect();

    let (sender, receiver) = channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = sender.send(event);
    })
    .map_err(to_io)?;
    watcher.watch(root, RecursiveMode::Recursive).map_err(to_io)?;

    loop {
        match receiver.recv_timeout(POLL_INTERVAL) {
            Ok(event) => {
                event.map_err(to_io)?;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "session watch channel closed",
                ));
            }
        }
        if let Some(found) = find_new_session(root, &known) {
            return Ok(found);
        }
    }
}

fn to_io(error: notify::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error)
}
