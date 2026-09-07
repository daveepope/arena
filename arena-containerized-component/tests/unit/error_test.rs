use arena_container::image::{pull_image, ImagePullClient};
use arena_containerized_component::error::ContainerizedComponentBuildError;
use bollard::auth::DockerCredentials;
use futures::stream::{self, BoxStream, StreamExt};

struct FailingImagePullClient;

impl ImagePullClient for FailingImagePullClient {
    fn pull(
        &self,
        _image: &str,
        _platform: &str,
        _credentials: Option<DockerCredentials>,
    ) -> BoxStream<'_, Result<String, String>> {
        stream::iter(vec![Err("boom".to_string())]).boxed()
    }

    async fn image_present_locally(&self, _image: &str, _platform: &str) -> bool {
        false
    }
}

#[test]
fn display_invalid_configuration_returns_message() {
    let err = ContainerizedComponentBuildError::InvalidConfiguration("bad config".to_string());
    assert_eq!(err.to_string(), "bad config");
}

#[test]
fn display_runtime_unavailable_returns_message() {
    let err = ContainerizedComponentBuildError::RuntimeUnavailable("no docker".to_string());
    assert_eq!(err.to_string(), "no docker");
}

#[test]
fn display_image_build_returns_identifier_and_message() {
    let err = ContainerizedComponentBuildError::ImageBuild {
        identifier: "web".to_string(),
        message: "bad Dockerfile".to_string(),
    };
    assert_eq!(err.to_string(), "web: image build failed: bad Dockerfile");
}

#[tokio::test]
async fn from_image_pull_error_wraps_display_message() {
    let pull_err = pull_image("web", "redis:8-alpine", "linux/amd64", &FailingImagePullClient)
        .await
        .expect_err("pull should fail");
    let expected = pull_err.to_string();

    let err: ContainerizedComponentBuildError = pull_err.into();

    assert_eq!(err.to_string(), expected);
}
