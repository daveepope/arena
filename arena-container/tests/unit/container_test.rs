use arena_container::container::{last_log_line, start_failure_message};
use testcontainers_modules::testcontainers::core::error::WaitContainerError;
use testcontainers_modules::testcontainers::TestcontainersError;

#[test]
fn last_log_line_trailing_blank_lines_returns_last_written_line() {
    let lines: Vec<&[u8]> = vec![b"starting", b"listening on 1433", b"   ", b""];

    assert_eq!(last_log_line(&lines).as_deref(), Some("listening on 1433"));
}

#[test]
fn last_log_line_only_blank_lines_returns_none() {
    let lines: Vec<&[u8]> = vec![b"", b"   "];

    assert_eq!(last_log_line(&lines), None);
}

#[test]
fn last_log_line_long_line_is_truncated() {
    let long = "x".repeat(400);
    let lines = vec![long.into_bytes()];

    let line = last_log_line(&lines).expect("a line");

    assert_eq!(line.chars().count(), 163);
    assert!(line.ends_with("..."));
}

#[test]
fn start_failure_message_startup_timeout_names_the_readiness_budget() {
    let error = TestcontainersError::WaitContainer(WaitContainerError::StartupTimeout);

    let message = start_failure_message("mssql", &error);

    assert_eq!(
        message,
        "mssql container failed to start: the container did not become ready within its startup budget"
    );
}

#[test]
fn start_failure_message_unhealthy_container_names_the_health_state() {
    let error = TestcontainersError::WaitContainer(WaitContainerError::Unhealthy);

    let message = start_failure_message("postgres", &error);

    assert_eq!(
        message,
        "postgres container failed to start: the container reported itself unhealthy"
    );
}

#[test]
fn start_failure_message_unexpected_exit_code_names_the_code() {
    let error = TestcontainersError::WaitContainer(WaitContainerError::UnexpectedExitCode {
        expected: 0,
        actual: Some(1),
    });

    let message = start_failure_message("kafka", &error);

    assert_eq!(
        message,
        "kafka container failed to start: the container exited with code 1"
    );
}

#[test]
fn start_failure_message_unknown_exit_code_omits_the_code() {
    let error = TestcontainersError::WaitContainer(WaitContainerError::UnexpectedExitCode {
        expected: 0,
        actual: None,
    });

    let message = start_failure_message("kafka", &error);

    assert_eq!(
        message,
        "kafka container failed to start: the container exited with an unknown exit code"
    );
}

#[test]
fn start_failure_message_other_error_falls_back_to_its_description() {
    let error = TestcontainersError::other("no runtime available");

    let message = start_failure_message("smtp", &error);

    assert_eq!(
        message,
        "smtp container failed to start: other error: no runtime available"
    );
}
