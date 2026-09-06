use arena_containerized_component::containerized_component::ContainerizedComponent;
use arena_containerized_component::error::ContainerizedComponentBuildError;

#[tokio::test]
async fn from_image_unknown_repository_returns_image_pull_error() {
    let result = ContainerizedComponent::from_image(
        "build-error-probe",
        "arena-nonexistent-repo-89f3c1e2/does-not-exist:latest",
    )
    .build()
    .await;

    let err = result
        .err()
        .expect("build should fail for a nonexistent image repository");

    assert!(matches!(err, ContainerizedComponentBuildError::ImagePull(_)));
}

#[tokio::test]
async fn build_from_invalid_containerfile_returns_image_build_error() {
    let result = ContainerizedComponent::builder("build-error-probe", "NOT A VALID DOCKERFILE INSTRUCTION")
        .build()
        .await;

    let err = result
        .err()
        .expect("build should fail for an invalid Containerfile");

    assert!(matches!(
        err,
        ContainerizedComponentBuildError::ImageBuild { .. }
    ));
}
