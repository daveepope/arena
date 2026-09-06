use super::fault::Subject;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Starting,
    Stopping,
    ForcedTeardown,
    RunningPlaybook,
}

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Stopping => "stopping",
            Self::ForcedTeardown => "being forcibly stopped",
            Self::RunningPlaybook => "running",
        }
    }
}

pub fn readiness_failed(cause: impl AsRef<str>) -> String {
    format!("readiness check failed: {}", cause.as_ref())
}

pub fn readiness_failed_for_target(target: impl AsRef<str>, cause: impl AsRef<str>) -> String {
    format!(
        "readiness check failed for target {}: {}",
        target.as_ref(),
        cause.as_ref()
    )
}

pub fn stop_did_not_complete() -> String {
    "stop did not complete".to_string()
}

pub fn child_start_failed(subject: Subject) -> String {
    format!("child {} failed to start", subject.as_str())
}

pub fn forced_teardown_unconfirmed() -> String {
    "forced teardown could not confirm the subject was removed".to_string()
}

pub fn panicked_while(phase: Phase, panic_text: impl AsRef<str>) -> String {
    format!("panicked while {}: {}", phase.as_str(), panic_text.as_ref())
}

pub fn match_panicked_while(index: usize, phase: Phase, panic_text: impl AsRef<str>) -> String {
    format!(
        "match {index} panicked while {}: {}",
        phase.as_str(),
        panic_text.as_ref()
    )
}

pub fn unexplained_after_teardown(
    subject: Subject,
    id: impl AsRef<str>,
    state: impl AsRef<str>,
) -> String {
    format!(
        "{} '{}' is {} after forced teardown and reported no fault",
        subject.as_str(),
        id.as_ref(),
        state.as_ref()
    )
}

