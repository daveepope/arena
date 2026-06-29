use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use arbiter::cursor::{
    find_new_session, latest_session_in, newest_session, scan_sessions, session_root,
};

fn temp_root(tag: &str) -> PathBuf {
    let unique = format!(
        "arbiter-sessions-{tag}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    );
    let root = std::env::temp_dir().join(unique);
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_session(root: &Path, id: &str) -> PathBuf {
    let directory = root.join(id);
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{id}.jsonl"));
    fs::write(&path, "{}").unwrap();
    path
}

#[test]
fn session_root_maps_workspace_to_cursor_layout() {
    let root = session_root(Path::new("/home/dave"), Path::new("/home/dave/repos/arena"));
    assert_eq!(
        root,
        Path::new("/home/dave/.cursor/projects/home-dave-repos-arena/agent-transcripts")
    );
}

#[test]
fn scan_sessions_collects_only_jsonl_files() {
    let root = temp_root("scan");
    let expected = write_session(&root, "alpha");
    fs::write(root.join("alpha").join("notes.txt"), "ignore me").unwrap();

    let found = scan_sessions(&root);

    assert_eq!(found, vec![expected]);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn find_new_session_returns_path_absent_from_known() {
    let root = temp_root("find-new");
    let known: HashSet<PathBuf> = scan_sessions(&root).into_iter().collect();
    let fresh = write_session(&root, "fresh");

    assert_eq!(find_new_session(&root, &known), Some(fresh));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn find_new_session_returns_none_when_nothing_new() {
    let root = temp_root("find-none");
    write_session(&root, "existing");
    let known: HashSet<PathBuf> = scan_sessions(&root).into_iter().collect();

    assert_eq!(find_new_session(&root, &known), None);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn newest_session_returns_most_recently_modified() {
    let older = PathBuf::from("/sessions/older.jsonl");
    let newer = PathBuf::from("/sessions/newer.jsonl");
    let base = SystemTime::UNIX_EPOCH;
    let dated = vec![
        (base + Duration::from_secs(10), newer.clone()),
        (base + Duration::from_secs(1), older),
    ];

    assert_eq!(newest_session(dated), Some(newer));
}

#[test]
fn newest_session_returns_none_when_empty() {
    assert_eq!(newest_session(Vec::new()), None);
}

#[test]
fn latest_session_in_errors_when_no_sessions() {
    let root = temp_root("latest-empty");

    assert!(latest_session_in(&root).is_err());
    fs::remove_dir_all(&root).ok();
}

#[test]
fn latest_session_in_returns_single_existing_session() {
    let root = temp_root("latest-single");
    let only = write_session(&root, "only");

    assert_eq!(latest_session_in(&root).unwrap(), only);
    fs::remove_dir_all(&root).ok();
}
