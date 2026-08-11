use std::io::Read;
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

fn tar_entry_names(tar_bytes: &[u8]) -> Vec<String> {
    let mut archive = tar::Archive::new(tar_bytes);
    archive
        .entries()
        .expect("read tar entries")
        .map(|entry| {
            entry
                .expect("read tar entry")
                .path()
                .expect("entry path")
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[test]
fn create_tar_without_build_context_contains_only_containerfile() {
    let tar_bytes =
        arena_container::build_context::create_tar("test-component", "FROM alpine:3.19\n", None);

    let names = tar_entry_names(&tar_bytes);

    assert_eq!(names, vec![".arena.Dockerfile".to_string()]);
}

#[test]
fn create_tar_includes_containerfile_content() {
    let containerfile = "FROM alpine:3.19\nCMD [\"sleep\", \"5\"]\n";
    let tar_bytes =
        arena_container::build_context::create_tar("test-component", containerfile, None);

    let mut archive = tar::Archive::new(tar_bytes.as_slice());
    let mut entries = archive.entries().expect("read tar entries");
    let mut entry = entries.next().expect("first entry").expect("read entry");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read content");

    assert_eq!(content, containerfile);
}

#[test]
fn create_tar_with_build_context_includes_files_and_skips_ignored_dirs() {
    let base_dir = std::env::temp_dir().join(unique_dir_name("arena-build-context-test"));
    std::fs::create_dir_all(&base_dir).expect("create base dir");

    std::fs::write(base_dir.join("app.txt"), b"hello").expect("write app file");

    let sub_dir = base_dir.join("src");
    std::fs::create_dir(&sub_dir).expect("create src dir");
    std::fs::write(sub_dir.join("main.rs"), b"fn main() {}").expect("write nested file");

    let git_dir = base_dir.join(".git");
    std::fs::create_dir(&git_dir).expect("create .git dir");
    std::fs::write(git_dir.join("HEAD"), b"ref: refs/heads/main").expect("write git file");

    let target_dir = base_dir.join("target");
    std::fs::create_dir(&target_dir).expect("create target dir");
    std::fs::write(target_dir.join("build.log"), b"log").expect("write target file");

    std::fs::write(base_dir.join(".env"), b"SECRET=1").expect("write hidden file");

    let tar_bytes = arena_container::build_context::create_tar(
        "test-component",
        "FROM alpine:3.19\n",
        Some(&base_dir),
    );

    std::fs::remove_dir_all(&base_dir).expect("clean up temp dir");

    let names = tar_entry_names(&tar_bytes);

    assert!(names.contains(&".arena.Dockerfile".to_string()));
    assert!(names.contains(&"app.txt".to_string()));
    assert!(names.iter().any(|n| n == "src" || n == "src/"));
    assert!(names.contains(&"src/main.rs".to_string()));
    assert!(!names.iter().any(|n| n.contains(".git")));
    assert!(!names.iter().any(|n| n.contains("target")));
    assert!(!names.iter().any(|n| n.contains(".env")));
}
