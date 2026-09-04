use bollard::query_parameters::{InspectContainerOptions, RemoveContainerOptionsBuilder};
use bollard::Docker;

/// Force-removes a Docker container by name.
///
/// This is intended to be called before creating a new container with a fixed
/// name, so that leftover containers from a previous (possibly crashed) run
/// do not cause a 409 Conflict.
///
/// If the container does not exist this is a silent no-op.
pub async fn try_remove_existing_container(name: &str) {
    let Some(docker) = Docker::connect_with_defaults().ok() else {
        tracing::debug!(container = %name, "docker unavailable; skip container remove");
        return;
    };

    let remove_options = RemoveContainerOptionsBuilder::default().force(true).build();

    match docker.remove_container(name, Some(remove_options)).await {
        Ok(_) => tracing::debug!(container = %name, "removed existing container"),
        Err(_) => tracing::debug!(
            container = %name,
            "no existing container to remove"
        ),
    }
}

pub async fn is_container_running(id_or_name: &str) -> bool {
    let Some(docker) = Docker::connect_with_defaults().ok() else {
        return true;
    };

    match docker
        .inspect_container(id_or_name, None::<InspectContainerOptions>)
        .await
    {
        Ok(details) => details
            .state
            .and_then(|state| state.running)
            .unwrap_or(false),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => false,
        Err(_) => true,
    }
}

pub fn start_failure_message(
    dependency: &str,
    error: &testcontainers_modules::testcontainers::TestcontainersError,
) -> String {
    use testcontainers_modules::testcontainers::core::logs::WaitLogError;
    use testcontainers_modules::testcontainers::core::error::WaitContainerError;
    use testcontainers_modules::testcontainers::TestcontainersError;

    let cause = match error {
        TestcontainersError::WaitContainer(WaitContainerError::WaitLog(
            WaitLogError::EndOfStream(lines),
        )) => match last_log_line(lines) {
            Some(line) => format!("the container exited during startup, last output: {line}"),
            None => "the container exited during startup without output".to_string(),
        },
        TestcontainersError::WaitContainer(WaitContainerError::StartupTimeout) => {
            "the container did not become ready within its startup budget".to_string()
        }
        TestcontainersError::WaitContainer(WaitContainerError::Unhealthy) => {
            "the container reported itself unhealthy".to_string()
        }
        TestcontainersError::WaitContainer(WaitContainerError::UnexpectedExitCode {
            actual,
            ..
        }) => format!("the container exited with code {actual:?}"),
        TestcontainersError::Client(client_error) => {
            format!("the container runtime rejected the request: {client_error}")
        }
        other => other.to_string(),
    };

    format!("{dependency} container failed to start: {cause}")
}

pub fn last_log_line<B: AsRef<[u8]>>(lines: &[B]) -> Option<String> {
    const MAX_LEN: usize = 160;

    let line = lines
        .iter()
        .rev()
        .map(|line| String::from_utf8_lossy(line.as_ref()).trim().to_string())
        .find(|line| !line.is_empty())?;

    if line.chars().count() > MAX_LEN {
        Some(line.chars().take(MAX_LEN).collect::<String>() + "...")
    } else {
        Some(line)
    }
}
