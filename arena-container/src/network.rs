use bollard::models::NetworkCreateRequest;
use bollard::Docker;

/// Ensures a Docker bridge network with the given name exists.
///
/// If the network already exists this is a no-op.
/// If it does not exist it will be created as a `bridge` network.
fn docker_client() -> Option<Docker> {
    Docker::connect_with_defaults().ok()
}

pub async fn ensure_network_exists(name: &str) {
    let Some(docker) = docker_client() else {
        panic!("failed to connect to Docker daemon");
    };

    match docker
        .inspect_network(
            name,
            None::<bollard::query_parameters::InspectNetworkOptions>,
        )
        .await
    {
        Ok(_) => {
            tracing::debug!(network = %name, "network already exists");
            return;
        }
        Err(_) => {
            tracing::debug!(network = %name, "network missing, creating");
        }
    }

    let config = NetworkCreateRequest {
        name: name.to_string(),
        driver: Some("bridge".to_string()),
        ..Default::default()
    };

    match docker.create_network(config).await {
        Ok(_) => tracing::debug!(network = %name, "network created"),
        Err(e) => {
            if docker
                .inspect_network(
                    name,
                    None::<bollard::query_parameters::InspectNetworkOptions>,
                )
                .await
                .is_ok()
            {
                tracing::debug!(
                    network = %name,
                    "network created concurrently by another caller"
                );
            } else {
                panic!("failed to create docker network '{}': {}", name, e);
            }
        }
    }
}

/// Removes a Docker network by name. Silently ignores errors (e.g. network
/// doesn't exist or still has connected containers).
pub async fn remove_network(name: &str) {
    let Some(docker) = docker_client() else {
        tracing::debug!(network = %name, "docker unavailable; skip network remove");
        return;
    };

    match docker.remove_network(name).await {
        Ok(_) => tracing::debug!(network = %name, "network removed"),
        Err(e) => tracing::debug!(
            network = %name,
            error = %e,
            "network remove skipped or failed"
        ),
    }
}
