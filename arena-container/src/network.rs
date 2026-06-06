use bollard::models::NetworkCreateRequest;
use bollard::Docker;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static NETWORK_REFS: OnceLock<Mutex<HashMap<String, usize>>> = OnceLock::new();

fn network_refs() -> &'static Mutex<HashMap<String, usize>> {
    NETWORK_REFS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn docker_client() -> Option<Docker> {
    Docker::connect_with_defaults().ok()
}

async fn create_network_if_missing(name: &str) {
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
                    "network already created"
                );
            } else {
                panic!("failed to create docker network '{}': {}", name, e);
            }
        }
    }
}

async fn remove_network_when_unused(name: &str) {
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

pub async fn ensure_network_exists(name: &str) {
    create_network_if_missing(name).await;
    let mut refs = network_refs().lock().expect("network ref lock");
    *refs.entry(name.to_string()).or_insert(0) += 1;
}

pub async fn remove_network(name: &str) {
    let should_remove = {
        let mut refs = network_refs().lock().expect("network ref lock");
        match refs.get_mut(name) {
            Some(count) if *count > 1 => {
                *count -= 1;
                false
            }
            Some(_) => {
                refs.remove(name);
                true
            }
            None => false,
        }
    };
    if should_remove {
        remove_network_when_unused(name).await;
    }
}
