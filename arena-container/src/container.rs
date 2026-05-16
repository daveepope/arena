use bollard::query_parameters::RemoveContainerOptionsBuilder;
use bollard::Docker;

/// Force-removes a Docker container by name.
///
/// This is intended to be called before creating a new container with a fixed
/// name, so that leftover containers from a previous (possibly crashed) run
/// do not cause a 409 Conflict.
///
/// If the container does not exist this is a silent no-op.
pub async fn try_remove_existing_container(name: &str) {
    let docker = Docker::connect_with_defaults().expect("failed to connect to Docker daemon");

    let remove_options = RemoveContainerOptionsBuilder::default().force(true).build();

    match docker.remove_container(name, Some(remove_options)).await {
        Ok(_) => tracing::debug!(container = %name, "removed existing container"),
        Err(_) => tracing::debug!(
            container = %name,
            "no existing container to remove"
        ),
    }
}
