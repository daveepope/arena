use arena_executable_component::platform::resolve_executable_extension;
use std::path::PathBuf;

fn unique_temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "arena-executable-component-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[cfg(windows)]
#[test]
fn resolve_executable_extension_barepath_missing_appendsplatformextension() {
    let dir = unique_temp_dir();
    let with_extension = dir.join("probe.exe");
    std::fs::write(&with_extension, b"").expect("write probe file");
    let bare_path = dir.join("probe");

    let resolved = resolve_executable_extension(bare_path);

    assert_eq!(resolved, with_extension);
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(windows)]
#[test]
fn resolve_executable_extension_barepath_missing_returnsoriginal_whennovariant() {
    let dir = unique_temp_dir();
    let bare_path = dir.join("does-not-exist");

    let resolved = resolve_executable_extension(bare_path.clone());

    assert_eq!(resolved, bare_path);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn resolve_executable_extension_pathalreadyexists_returnsunchanged() {
    let dir = unique_temp_dir();
    let path = dir.join("already-there");
    std::fs::write(&path, b"").expect("write file");

    let resolved = resolve_executable_extension(path.clone());

    assert_eq!(resolved, path);
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(not(windows))]
#[test]
fn resolve_executable_extension_nonwindows_returnsunchanged() {
    let path = PathBuf::from("/does/not/matter");

    let resolved = resolve_executable_extension(path.clone());

    assert_eq!(resolved, path);
}
