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
    let candidates: &[&str] = if cfg!(windows) {
        &["python", "python3", "py"]
    } else {
        &["python3", "python"]
    };

    let scripts_dir = script_path
        .parent()
        .expect("write_default_images_rs.py path has a parent");

    let mut failures = Vec::new();
    for candidate in candidates {
        match Command::new(candidate)
            .arg(script_path)
            .arg(toml_path)
            .arg(out_path)
            .env("PYTHONPATH", scripts_dir)
            .status()
        {
            Ok(status) if status.success() => return,
            Ok(status) => failures.push(format!("{candidate}: exited with {status}")),
            Err(e) => failures.push(format!("{candidate}: {e}")),
        }
    }

    panic!(
        "failed to run {} with any of {:?} (is a Python interpreter on PATH?): {:?}",
        script_path.display(),
        candidates,
        failures
    );
}
