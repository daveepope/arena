use arena_container::identifier::build;
use arena_container::network::{ensure_network_exists, remove_network};
use bollard::Docker;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_network_name() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    build("arena-container", &format!("lifecycle-{ts}"))
}

async fn network_is_present(name: &str) -> bool {
    let Ok(docker) = Docker::connect_with_defaults() else {
        panic!("docker unavailable");
    };
    docker
        .inspect_network(
            name,
            None::<bollard::query_parameters::InspectNetworkOptions>,
        )
        .await
        .is_ok()
}

#[tokio::test]
async fn ensure_network_exists_creates_then_remove_network_when_unused_tears_down() {
    let name = unique_network_name();

    ensure_network_exists(&name).await;
    assert!(
        network_is_present(&name).await,
        "expected network to exist after ensure on missing name"
    );

    ensure_network_exists(&name).await;
    assert!(
        network_is_present(&name).await,
        "expected network to remain after second acquire"
    );

    remove_network(&name).await;
    assert!(
        network_is_present(&name).await,
        "expected network to remain while one acquire is still held"
    );

    remove_network(&name).await;
    assert!(
        !network_is_present(&name).await,
        "expected network removed after final release when unused"
    );
}
