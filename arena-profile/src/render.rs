use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process::Command;

#[derive(Debug)]
pub enum RenderError {
    Io(std::io::Error),
    Inferno(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::Io(e) => write!(f, "io error rendering flamegraph report: {e}"),
            RenderError::Inferno(msg) => write!(f, "failed to render flamegraph: {msg}"),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<std::io::Error> for RenderError {
    fn from(e: std::io::Error) -> Self {
        RenderError::Io(e)
    }
}

pub fn render_folded_to_html(folded: impl Read, output_path: &Path) -> Result<(), RenderError> {
    let mut html = std::fs::File::create(output_path)?;
    write!(
        html,
        "<!DOCTYPE html>\n<html>\n<head><meta charset=\"utf-8\"><title>CPU Profile</title></head>\n<body>\n"
    )?;

    let mut opts = inferno::flamegraph::Options::default();
    inferno::flamegraph::from_reader(&mut opts, BufReader::new(folded), &mut html)
        .map_err(|e| RenderError::Inferno(e.to_string()))?;

    write!(html, "\n</body>\n</html>\n")?;
    Ok(())
}

pub fn open_report(path: &Path) -> std::io::Result<()> {
    if cfg!(target_os = "macos") {
        Command::new("open").arg(path).status()?;
        return Ok(());
    }
    if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", ""]).arg(path).status()?;
        return Ok(());
    }
    if is_wsl() {
        let win_path = Command::new("wslpath").arg("-w").arg(path).output()?;
        let win_path = String::from_utf8_lossy(&win_path.stdout).trim().to_string();
        // explorer.exe frequently returns a non-zero exit status even when it
        // successfully opens the file, so its status is intentionally not checked.
        let _ = Command::new("explorer.exe").arg(win_path).status();
        return Ok(());
    }
    Command::new("xdg-open").arg(path).status()?;
    Ok(())
}

fn is_wsl() -> bool {
    std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::fs::read_to_string("/proc/version")
            .map(|v| v.to_lowercase().contains("microsoft"))
            .unwrap_or(false)
}
