use bollard::query_parameters::ListContainersOptionsBuilder;
use bollard::Docker;

pub fn unique_tag(prefix: &str) -> String {
    format!(
        "{prefix}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos()
    )
}

pub async fn find_container_mounts(
    docker: &Docker,
    image_tag: &str,
) -> Vec<bollard::models::MountPoint> {
    let containers = docker
        .list_containers(Some(
            ListContainersOptionsBuilder::default().all(true).build(),
        ))
        .await
        .expect("list containers");

    let container = containers
        .into_iter()
        .find(|c| c.image.as_deref() == Some(image_tag))
        .expect("container with matching image tag should exist");

    let inspect = docker
        .inspect_container(container.id.as_deref().expect("container id"), None)
        .await
        .expect("inspect container");

    inspect.mounts.unwrap_or_default()
}
