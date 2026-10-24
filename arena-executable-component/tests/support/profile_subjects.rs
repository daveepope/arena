use arena::component::RunnableComponent;
use arena::lifecycle::RunnableState;
use arena_executable_component::executable_component::ExecutableComponent;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const PASS_MILLIS: u64 = 500;
pub const PASS_BUDGET: Duration = Duration::from_secs(20);

const POLL_INTERVAL: Duration = Duration::from_millis(10);

pub fn subject_path(subject: &str) -> String {
    let path = PathBuf::from(subject);
    assert!(
        path.exists(),
        "{subject} is not present in the test's runfiles (cwd {:?})",
        std::env::current_dir()
    );
    path.to_string_lossy().into_owned()
}

pub fn marker_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "arena-executable-component-subject-{name}-{}",
        std::process::id()
    ))
}

pub async fn await_completed_pass(
    marker: &Path,
    component: &ExecutableComponent,
) -> Result<(), String> {
    let _ = std::fs::remove_file(marker);
    let deadline = Instant::now() + PASS_BUDGET;
    while Instant::now() < deadline {
        if marker.exists() {
            return Ok(());
        }
        if component.state() == RunnableState::Faulted {
            return Err("component faulted before completing a pass".to_string());
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
    Err(format!("subject did not complete a pass within {PASS_BUDGET:?}"))
}
