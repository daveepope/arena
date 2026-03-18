use bollard::Docker;
use bollard::models::NetworkCreateRequest;

/// Ensures a Docker bridge network with the given name exists.
///
/// If the network already exists this is a no-op.
/// If it does not exist it will be created as a `bridge` network.
pub async fn ensure_network_exists(name: &str) {
    let docker = Docker::connect_with_defaults()
        .expect("failed to connect to Docker daemon");

    match docker.inspect_network(name, None::<bollard::query_parameters::InspectNetworkOptions>).await {
        Ok(_) => {
            log::debug!("[arena-container] network '{}' already exists", name);
            return;
        }
        Err(_) => {
            log::debug!("[arena-container] network '{}' not found, creating", name);
        }
    }

    let config = NetworkCreateRequest {
        name: name.to_string(),
        driver: Some("bridge".to_string()),
        ..Default::default()
    };

    match docker.create_network(config).await {
        Ok(_) => log::info!("[arena-container] created network '{}'", name),
        Err(e) => {
            if docker.inspect_network(name, None::<bollard::query_parameters::InspectNetworkOptions>).await.is_ok() {
                log::debug!("[arena-container] network '{}' was created concurrently", name);
            } else {
                panic!("failed to create docker network '{}': {}", name, e);
            }
        }
    }
}

/// Removes a Docker network by name. Silently ignores errors (e.g. network
/// doesn't exist or still has connected containers).
pub async fn remove_network(name: &str) {
    let docker = Docker::connect_with_defaults()
        .expect("failed to connect to Docker daemon");

    match docker.remove_network(name).await {
        Ok(_) => log::info!("[arena-container] removed network '{}'", name),
        Err(e) => log::debug!("[arena-container] could not remove network '{}': {}", name, e),
    }
}
