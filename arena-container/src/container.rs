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
