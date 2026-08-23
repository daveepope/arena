use bollard::auth::DockerCredentials;
use bollard::models::ImageInspect;
use bollard::query_parameters::CreateImageOptionsBuilder;
use bollard::Docker;
use docker_credential::{CredentialRetrievalError, DockerCredential};
use futures::stream::BoxStream;
use futures::StreamExt;
use std::fmt;

const DOCKER_HUB_REGISTRY: &str = "https://index.docker.io/v1/";

pub trait ImagePullClient: Send + Sync {
    fn pull(
        &self,
        image: &str,
        platform: &str,
        credentials: Option<DockerCredentials>,
    ) -> BoxStream<'_, Result<String, String>>;

    fn image_present_locally(
        &self,
        image: &str,
        platform: &str,
    ) -> impl std::future::Future<Output = bool> + Send;
}

impl ImagePullClient for Docker {
    fn pull(
        &self,
        image: &str,
        platform: &str,
        credentials: Option<DockerCredentials>,
    ) -> BoxStream<'_, Result<String, String>> {
        let options = CreateImageOptionsBuilder::default()
            .from_image(image)
            .platform(platform)
            .build();

        self.create_image(Some(options), None, credentials)
            .map(|result| {
                result
                    .map(|info| info.status.unwrap_or_default())
                    .map_err(|e| e.to_string())
            })
            .boxed()
    }

    async fn image_present_locally(&self, image: &str, platform: &str) -> bool {
        match self.inspect_image(image).await {
            Ok(inspect) => image_matches_platform(&inspect, platform),
            Err(_) => false,
        }
    }
}

pub fn image_matches_platform(inspect: &ImageInspect, platform: &str) -> bool {
    let mut requested = platform.splitn(3, '/');
    let requested_os = requested.next();
    let requested_arch = requested.next();
    let requested_variant = requested.next();

    field_matches(requested_os, inspect.os.as_deref())
        && field_matches(requested_arch, inspect.architecture.as_deref())
        && field_matches(requested_variant, inspect.variant.as_deref())
}

fn field_matches(requested: Option<&str>, actual: Option<&str>) -> bool {
    match (requested, actual) {
        (Some(want), Some(have)) => want.eq_ignore_ascii_case(have),
        (Some(_), None) => false,
        (None, _) => true,
    }
}

pub fn registry_host(image: &str) -> String {
    match image.split_once('/') {
        Some((first, _rest)) if is_registry_host(first) => first.to_string(),
        _ => DOCKER_HUB_REGISTRY.to_string(),
    }
}

fn is_registry_host(segment: &str) -> bool {
    segment == "localhost" || segment.contains('.') || segment.contains(':')
}

pub async fn resolve_registry_credentials(image: &str) -> Option<DockerCredentials> {
    let host = registry_host(image);

    let outcome = tokio::task::spawn_blocking({
        let host = host.clone();
        move || docker_credential::get_credential(&host)
    })
    .await;

    match outcome {
        Ok(Ok(credential)) => Some(to_docker_credentials(credential, &host)),
        Ok(Err(
            CredentialRetrievalError::NoCredentialConfigured | CredentialRetrievalError::ConfigNotFound,
        )) => {
            tracing::debug!(
                registry = %host,
                phase = "registry_credentials_none",
                "no registry credentials configured, pulling anonymously",
            );
            None
        }
        Ok(Err(e)) => {
            tracing::warn!(
                registry = %host,
                error = %e,
                phase = "registry_credentials_error",
                "failed to resolve registry credentials, falling back to anonymous pull",
            );
            None
        }
        Err(join_err) => {
            tracing::warn!(
                registry = %host,
                error = %join_err,
                phase = "registry_credentials_error",
                "registry credential resolution task failed, falling back to anonymous pull",
            );
            None
        }
    }
}

pub fn to_docker_credentials(credential: DockerCredential, host: &str) -> DockerCredentials {
    match credential {
        DockerCredential::UsernamePassword(username, password) => DockerCredentials {
            username: Some(username),
            password: Some(password),
            serveraddress: Some(host.to_string()),
            ..Default::default()
        },
        DockerCredential::IdentityToken(token) => DockerCredentials {
            identitytoken: Some(token),
            serveraddress: Some(host.to_string()),
            ..Default::default()
        },
    }
}

#[derive(Debug)]
pub struct ImagePullError {
    identifier: String,
    message: String,
}

impl fmt::Display for ImagePullError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: image pull failed: {}", self.identifier, self.message)
    }
}

impl std::error::Error for ImagePullError {}

pub async fn pull_image(
    identifier: &str,
    image: &str,
    platform: &str,
    runtime_client: &impl ImagePullClient,
) -> Result<(), ImagePullError> {
    if runtime_client.image_present_locally(image, platform).await {
        tracing::debug!(
            component = %identifier,
            image = %image,
            phase = "image_pull_skipped",
            "image already present locally for the requested platform, skipping pull",
        );
        return Ok(());
    }

    tracing::debug!(
        component = %identifier,
        image = %image,
        phase = "image_pull_begin",
        "pulling container image",
    );

    let credentials = resolve_registry_credentials(image).await;
    let mut stream = runtime_client.pull(image, platform, credentials);

    while let Some(result) = stream.next().await {
        match result {
            Ok(status) => {
                if let Some(line) = pull_status_line(Some(&status)) {
                    tracing::debug!(
                        component = %identifier,
                        text = %line,
                        phase = "image_pull_stream",
                        "image pull output line",
                    );
                }
            }
            Err(e) => {
                return Err(ImagePullError {
                    identifier: identifier.to_string(),
                    message: e,
                });
            }
        }
    }

    tracing::debug!(
        component = %identifier,
        image = %image,
        phase = "image_pull_done",
        "container image pulled",
    );

    Ok(())
}

pub fn pull_status_line(status: Option<&str>) -> Option<&str> {
    let line = status?.trim_end();
    (!line.is_empty()).then_some(line)
}
