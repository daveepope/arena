use std::path::Path;

const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", ".idea", ".vscode", ".arena"];

pub fn create_tar(identifier: &str, containerfile: &str, build_context: Option<&Path>) -> Vec<u8> {
    let buf = Vec::new();
    let mut tar = tar::Builder::new(buf);

    let containerfile_bytes = containerfile.as_bytes();
    let mut header = tar::Header::new_ustar();
    header.set_size(containerfile_bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, ".arena.Dockerfile", containerfile_bytes)
        .expect("add containerfile to image build context");

    if let Some(context_path) = build_context {
        append_dir_recursive(&mut tar, context_path, context_path, identifier);
    }

    tar.into_inner().expect("finalize tar archive")
}

fn append_dir_recursive(
    tar: &mut tar::Builder<Vec<u8>>,
    base_path: &Path,
    current_path: &Path,
    identifier: &str,
) {
    let entries = match std::fs::read_dir(current_path) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!(
                component = %identifier,
                path = ?current_path,
                error = %e,
                "skipping unreadable directory",
            );
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with('.') || SKIP_DIRS.contains(&name_str.as_ref()) {
            continue;
        }

        let relative = match path.strip_prefix(base_path) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if metadata.is_dir() {
            let mut header = tar::Header::new_ustar();
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            header.set_mode(0o755);
            header.set_cksum();
            if let Err(e) = tar.append_data(&mut header, relative, &[] as &[u8]) {
                tracing::warn!(
                    component = %identifier,
                    path = ?relative,
                    error = %e,
                    "skipping directory archive entry",
                );
                continue;
            }
            append_dir_recursive(tar, base_path, &path, identifier);
        } else if metadata.is_file() {
            let content = match std::fs::read(&path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        component = %identifier,
                        path = ?relative,
                        error = %e,
                        "skipping unreadable file",
                    );
                    continue;
                }
            };
            let mut header = tar::Header::new_ustar();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            if let Err(e) = tar.append_data(&mut header, relative, content.as_slice()) {
                tracing::warn!(
                    component = %identifier,
                    path = ?relative,
                    error = %e,
                    "skipping tar file append failure",
                );
            }
        }
    }
}
