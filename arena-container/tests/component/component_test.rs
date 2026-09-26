use arena_container::container::{
    force_remove_container, is_container_running, try_remove_existing_container,
};
use arena_container::default_images::SMTP;
use arena_container::expiry::{
    remove_expired_containers, remove_expired_containers_if_enabled, EXPIRES_AT_LABEL, MODULE_LABEL,
};
use arena_container::identifier::build;
use arena_container::network::{ensure_network_exists, remove_network};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, CreateImageOptionsBuilder, ListContainersOptionsBuilder,
    RemoveContainerOptionsBuilder,
};
use bollard::Docker;
use futures::StreamExt;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SELECTIVITY_MODULE: &str = "arena-container-expiry-selectivity";
const DISABLED_MODULE: &str = "arena-container-expiry-disabled";
const ENABLED_MODULE: &str = "arena-container-expiry-enabled";

fn docker() -> Docker {
    Docker::connect_with_defaults().expect("container runtime should be reachable")
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn unique_name(prefix: &str) -> String {
    build("arena-container", &format!("{prefix}-{}", now_millis()))
}

async fn pull_probe_image(docker: &Docker) {
    let options = CreateImageOptionsBuilder::default()
        .from_image(SMTP.image)
        .tag(SMTP.tag)
        .build();
    let mut stream = docker.create_image(Some(options), None, None);
    while let Some(next) = stream.next().await {
        next.expect("probe image should pull");
    }
}

async fn create_probe_container(docker: &Docker, name: &str, labels: HashMap<String, String>) {
    let body = bollard::models::ContainerCreateBody {
        image: Some(format!("{}:{}", SMTP.image, SMTP.tag)),
        labels: Some(labels),
        ..Default::default()
    };
    let options = CreateContainerOptionsBuilder::default().name(name).build();
    docker
        .create_container(Some(options), body)
        .await
        .expect("probe container should be created");
}

fn labels(module: &str, expires_at: u128) -> HashMap<String, String> {
    HashMap::from([
        (MODULE_LABEL.to_string(), module.to_string()),
        (EXPIRES_AT_LABEL.to_string(), expires_at.to_string()),
    ])
}

async fn container_exists(docker: &Docker, name: &str) -> bool {
    let filters = HashMap::from([("name".to_string(), vec![name.to_string()])]);
    let options = ListContainersOptionsBuilder::new()
        .all(true)
        .filters(&filters)
        .build();
    docker
        .list_containers(Some(options))
        .await
        .expect("listing should succeed")
        .into_iter()
        .any(|found| {
            found
                .names
                .unwrap_or_default()
                .iter()
                .any(|candidate| candidate.trim_start_matches('/') == name)
        })
}

async fn remove_quietly(docker: &Docker, name: &str) {
    let options = RemoveContainerOptionsBuilder::default().force(true).build();
    let _ = docker.remove_container(name, Some(options)).await;
}

async fn network_is_present(name: &str) -> bool {
    docker()
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
    assert!(network_is_present(&name).await, "network should exist");

    remove_network(&name).await;
    assert!(!network_is_present(&name).await, "network should be removed");
}

#[tokio::test]
async fn remove_network_while_still_referenced_keeps_the_network() {
    let name = unique_network_name();

    ensure_network_exists(&name).await;
    ensure_network_exists(&name).await;
    remove_network(&name).await;

    assert!(
        network_is_present(&name).await,
        "network should survive while a reference remains"
    );

    remove_network(&name).await;
    assert!(!network_is_present(&name).await, "network should be removed");
}

fn unique_network_name() -> String {
    unique_name("network-lifecycle")
}

#[tokio::test]
async fn try_remove_existing_container_present_container_removes_it() {
    let docker = docker();
    pull_probe_image(&docker).await;
    let name = unique_name("remove-existing");
    create_probe_container(&docker, &name, HashMap::new()).await;
    assert!(container_exists(&docker, &name).await);

    try_remove_existing_container(&name).await;

    assert!(!container_exists(&docker, &name).await);
}

#[tokio::test]
async fn try_remove_existing_container_absent_container_is_a_noop() {
    let docker = docker();
    let name = unique_name("remove-absent");

    try_remove_existing_container(&name).await;

    assert!(!container_exists(&docker, &name).await);
}

#[tokio::test]
async fn force_remove_container_present_container_returns_true() {
    let docker = docker();
    pull_probe_image(&docker).await;
    let name = unique_name("force-remove");
    create_probe_container(&docker, &name, HashMap::new()).await;

    assert!(force_remove_container(&name).await);
    assert!(!container_exists(&docker, &name).await);
}

#[tokio::test]
async fn is_container_running_stopped_container_returns_false() {
    let docker = docker();
    pull_probe_image(&docker).await;
    let name = unique_name("is-running");
    create_probe_container(&docker, &name, HashMap::new()).await;

    let running = is_container_running(&name).await;
    remove_quietly(&docker, &name).await;

    assert!(!running, "a created but unstarted container is not running");
}

#[tokio::test]
async fn remove_expired_containers_expired_container_of_that_module_removes_only_it() {
    let docker = docker();
    pull_probe_image(&docker).await;
    let past = now_millis() - 60_000;
    let future = now_millis() + 600_000;

    let expired = unique_name("expired");
    let unexpired = unique_name("unexpired");
    let other_module = unique_name("other-module");
    let unlabelled = unique_name("unlabelled");

    create_probe_container(&docker, &expired, labels(SELECTIVITY_MODULE, past)).await;
    create_probe_container(&docker, &unexpired, labels(SELECTIVITY_MODULE, future)).await;
    create_probe_container(&docker, &other_module, labels("arena-container-other", past)).await;
    create_probe_container(&docker, &unlabelled, HashMap::new()).await;

    remove_expired_containers(SELECTIVITY_MODULE).await;

    let expired_gone = !container_exists(&docker, &expired).await;
    let unexpired_kept = container_exists(&docker, &unexpired).await;
    let other_module_kept = container_exists(&docker, &other_module).await;
    let unlabelled_kept = container_exists(&docker, &unlabelled).await;

    for name in [&unexpired, &other_module, &unlabelled, &expired] {
        remove_quietly(&docker, name).await;
    }

    assert!(expired_gone, "an expired container should be removed");
    assert!(unexpired_kept, "an unexpired container should be kept");
    assert!(other_module_kept, "another module's container should be kept");
    assert!(unlabelled_kept, "an unlabelled container should be kept");
}

#[tokio::test]
async fn remove_expired_containers_if_enabled_disabled_expiry_keeps_expired_containers() {
    let docker = docker();
    pull_probe_image(&docker).await;
    let name = unique_name("disabled-sweep");
    create_probe_container(&docker, &name, labels(DISABLED_MODULE, now_millis() - 60_000)).await;

    remove_expired_containers_if_enabled(DISABLED_MODULE, None).await;

    let kept = container_exists(&docker, &name).await;
    remove_quietly(&docker, &name).await;

    assert!(kept, "a disabled sweep should not remove anything");
}

#[tokio::test]
async fn remove_expired_containers_if_enabled_enabled_expiry_removes_expired_containers() {
    let docker = docker();
    pull_probe_image(&docker).await;
    let name = unique_name("enabled-sweep");
    create_probe_container(&docker, &name, labels(ENABLED_MODULE, now_millis() - 60_000)).await;

    remove_expired_containers_if_enabled(ENABLED_MODULE, Some(Duration::from_secs(300))).await;

    let removed = !container_exists(&docker, &name).await;
    remove_quietly(&docker, &name).await;

    assert!(removed, "an enabled sweep should remove an expired container");
}

#[tokio::test]
async fn remove_expired_containers_if_enabled_called_twice_sweeps_once_per_interval() {
    let docker = docker();
    pull_probe_image(&docker).await;
    let module = "arena-container-expiry-gated";
    let first = unique_name("gated-first");

    create_probe_container(&docker, &first, labels(module, now_millis() - 60_000)).await;
    remove_expired_containers_if_enabled(module, Some(Duration::from_secs(300))).await;
    let first_removed = !container_exists(&docker, &first).await;

    let second = unique_name("gated-second");
    create_probe_container(&docker, &second, labels(module, now_millis() - 60_000)).await;
    remove_expired_containers_if_enabled(module, Some(Duration::from_secs(300))).await;
    let second_kept = container_exists(&docker, &second).await;

    remove_quietly(&docker, &second).await;
    remove_quietly(&docker, &first).await;

    assert!(first_removed, "the first sweep should claim and remove");
    assert!(
        second_kept,
        "a second sweep inside the interval should be skipped"
    );
}
