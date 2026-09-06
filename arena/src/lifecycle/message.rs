use super::fault::Subject;

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

pub fn start_failed() -> String {
    "failed to start".to_string()
}

pub fn stop_failed() -> String {
    "failed to stop".to_string()
}

pub fn playbook_failed() -> String {
    "failed to run".to_string()
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

pub fn reset_failed() -> String {
    "failed to reset".to_string()
}
