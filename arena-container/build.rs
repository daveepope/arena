use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .expect("arena-container manifest dir has a parent")
        .to_path_buf();
    let toml_path = repo_root.join("container_defaults.toml");
    let script_path = repo_root.join("scripts/write_default_images_rs.py");
    let out_path = manifest_dir.join("src/default_images.rs");

    println!("cargo:rerun-if-changed={}", toml_path.display());
    println!("cargo:rerun-if-changed={}", script_path.display());

    run_generator(&script_path, &toml_path, &out_path);
}

fn run_generator(script_path: &Path, toml_path: &Path, out_path: &Path) {
    let status = Command::new("python3")
        .arg(script_path)
        .arg(toml_path)
        .arg(out_path)
        .status()
        .unwrap_or_else(|e| {
            panic!(
                "failed to run {} (is python3 on PATH?): {e}",
                script_path.display()
            )
        });

    if !status.success() {
        panic!(
            "{} exited with {status} while generating {}",
            script_path.display(),
            out_path.display()
        );
    }
}
