use std::path::PathBuf;

pub fn resolve(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        let current_dir = std::env::current_dir().expect("get current directory");

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
    }
}
