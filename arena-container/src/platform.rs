use crate::image::{inspected_platform_name, resolve_registry_credentials};
use bollard::models::OciPlatform;
use bollard::Docker;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub const FALLBACK_PLATFORM: &str = "linux/amd64";

static RESOLVED_PLATFORMS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn resolved_platforms() -> &'static Mutex<HashMap<String, String>> {
    RESOLVED_PLATFORMS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn docker_platform() -> String {
    platform_for_arch(std::env::consts::ARCH)
}

pub fn platform_for_arch(arch: &str) -> String {
    let arch = match arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    };
    format!("linux/{arch}")
}

pub trait ImagePlatformClient: Send + Sync {
    fn published_platforms(
        &self,
        image_reference: &str,
    ) -> impl std::future::Future<Output = Result<Vec<String>, String>> + Send;

    fn local_platform(
        &self,
        image_reference: &str,
    ) -> impl std::future::Future<Output = Option<String>> + Send;
}

pub struct RuntimePlatformClient;

impl ImagePlatformClient for RuntimePlatformClient {
    async fn published_platforms(&self, image_reference: &str) -> Result<Vec<String>, String> {
        let runtime_client = Docker::connect_with_defaults().map_err(|e| e.to_string())?;
        let credentials = resolve_registry_credentials(image_reference).await;

        runtime_client
            .inspect_registry_image(image_reference, credentials)
            .await
            .map(|inspect| published_platform_names(&inspect.platforms))
            .map_err(|e| e.to_string())
    }

    async fn local_platform(&self, image_reference: &str) -> Option<String> {
        let runtime_client = Docker::connect_with_defaults().ok()?;
        let inspect = runtime_client.inspect_image(image_reference).await.ok()?;
        inspected_platform_name(&inspect)
    }
}

pub fn published_platform_names(platforms: &[OciPlatform]) -> Vec<String> {
    platforms.iter().filter_map(platform_name).collect()
}

pub fn platform_name(platform: &OciPlatform) -> Option<String> {
    let os = platform.os.as_deref()?;
    let architecture = platform.architecture.as_deref()?;
    Some(format!("{os}/{architecture}"))
}

pub async fn resolve_platform(image_name: &str, image_tag: &str) -> String {
    resolve_platform_for_reference(&format!("{image_name}:{image_tag}")).await
}

pub async fn resolve_platform_for_reference(image_reference: &str) -> String {
    resolve_platform_with(image_reference, &docker_platform(), &RuntimePlatformClient).await
}

pub async fn resolve_platform_with(
    image_reference: &str,
    host_platform: &str,
    runtime_client: &impl ImagePlatformClient,
) -> String {
    if host_platform == FALLBACK_PLATFORM {
        return host_platform.to_string();
    }

    let cache_key = cache_key(image_reference, host_platform);
    if let Some(resolved) = cached_platform(&cache_key) {
        return resolved;
    }

    match select_platform(image_reference, host_platform, runtime_client).await {
        Some(resolved) => {
            cache_platform(&cache_key, &resolved);
            resolved
        }
        None => host_platform.to_string(),
    }
}

pub async fn select_platform(
    image_reference: &str,
    host_platform: &str,
    runtime_client: &impl ImagePlatformClient,
) -> Option<String> {
    let published = match runtime_client.published_platforms(image_reference).await {
        Ok(published) => published,
        Err(error) => {
            tracing::debug!(
                image = %image_reference,
                error = %error,
                phase = "platform_lookup_failed",
                "could not read the platforms published for the image, falling back to the local image",
            );
            return runtime_client.local_platform(image_reference).await;
        }
    };

    if published.is_empty()
        || published
            .iter()
            .any(|platform| platform.eq_ignore_ascii_case(host_platform))
    {
        return Some(host_platform.to_string());
    }

    if published
        .iter()
        .any(|platform| platform.eq_ignore_ascii_case(FALLBACK_PLATFORM))
    {
        tracing::warn!(
            image = %image_reference,
            host_platform = %host_platform,
            selected_platform = %FALLBACK_PLATFORM,
            phase = "platform_fallback",
            "image publishes no host platform variant, running the linux/amd64 variant instead",
        );
        return Some(FALLBACK_PLATFORM.to_string());
    }

    Some(host_platform.to_string())
}

fn cache_key(image_reference: &str, host_platform: &str) -> String {
    format!("{host_platform}|{image_reference}")
}

fn cached_platform(cache_key: &str) -> Option<String> {
    resolved_platforms().lock().ok()?.get(cache_key).cloned()
}

fn cache_platform(cache_key: &str, platform: &str) {
    if let Ok(mut cache) = resolved_platforms().lock() {
        cache.insert(cache_key.to_string(), platform.to_string());
    }
}
