use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_dir_name(prefix: &str) -> String {
    format!(
        "{prefix}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos()
    )
}

#[test]
fn resolve_absolute_path_returns_unchanged() {
    let absolute = PathBuf::from("/tmp/some/absolute/path");

    let resolved = arena_container::path::resolve(absolute.clone());

    assert_eq!(resolved, absolute);
}

#[test]
fn resolve_relative_path_changes_behavior_with_current_dir() {
    let original_dir = std::env::current_dir().expect("get current dir");

    let not_found = arena_container::path::resolve(PathBuf::from(
        "arena-path-does-not-exist-anywhere",
    ));
    assert_eq!(
        not_found,
        original_dir.join("arena-path-does-not-exist-anywhere")
    );

    let base_dir = std::env::temp_dir().join(unique_dir_name("arena-path-resolve-test"));
    let nested = base_dir.join("nested");
    std::fs::create_dir_all(&nested).expect("create nested dir");
    let marker = nested.join("marker.txt");
    std::fs::write(&marker, b"content").expect("write marker file");

    std::env::set_current_dir(&nested).expect("set current dir");
    let found = arena_container::path::resolve(PathBuf::from("marker.txt"));
    std::env::set_current_dir(&original_dir).expect("restore current dir");

    assert!(found.exists());
    assert_eq!(found.file_name().unwrap(), "marker.txt");

    std::fs::remove_dir_all(&base_dir).expect("clean up temp dir");
}
