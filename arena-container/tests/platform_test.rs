use arena_container::platform::{
    docker_platform, platform_name, published_platform_names, resolve_platform_with,
    select_platform, PublishedPlatformClient,
};
use bollard::models::OciPlatform;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn docker_platform_returns_linux_prefixed_value() {
    assert!(docker_platform().starts_with("linux/"));
}

#[test]
fn docker_platform_maps_current_arch_to_docker_naming() {
    let expected_arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    };
    assert_eq!(docker_platform(), format!("linux/{expected_arch}"));
}

struct FakePublishedPlatformClient {
    published: Result<Vec<String>, String>,
    lookups: AtomicUsize,
}

fn setup_fake_client(published: Result<Vec<&str>, &str>) -> FakePublishedPlatformClient {
    FakePublishedPlatformClient {
        published: published
            .map(|platforms| platforms.into_iter().map(str::to_string).collect())
            .map_err(str::to_string),
        lookups: AtomicUsize::new(0),
    }
}

impl PublishedPlatformClient for FakePublishedPlatformClient {
    async fn published_platforms(&self, _image_reference: &str) -> Result<Vec<String>, String> {
        self.lookups.fetch_add(1, Ordering::SeqCst);
        self.published.clone()
    }
}

#[tokio::test]
async fn select_platform_published_variants_returns_runnable_platform() {
    let cases: Vec<(Result<Vec<&str>, &str>, &str, &str)> = vec![
        (Ok(vec!["linux/amd64", "linux/arm64"]), "linux/arm64", "linux/arm64"),
        (Ok(vec!["linux/amd64", "unknown/unknown"]), "linux/arm64", "linux/amd64"),
        (Ok(vec!["linux/arm/v7"]), "linux/arm64", "linux/arm64"),
        (Ok(vec![]), "linux/arm64", "linux/arm64"),
        (Err("registry unreachable"), "linux/arm64", "linux/arm64"),
        (Ok(vec!["linux/amd64"]), "linux/amd64", "linux/amd64"),
    ];

    for (published, host_platform, expected) in cases {
        let runtime_client = setup_fake_client(published.clone());

        let selected = select_platform("image:tag", host_platform, &runtime_client).await;

        assert_eq!(selected, expected, "published: {published:?}, host: {host_platform}");
    }
}

fn setup_oci_platform(os: Option<&str>, architecture: Option<&str>, variant: Option<&str>) -> OciPlatform {
    OciPlatform {
        os: os.map(str::to_string),
        architecture: architecture.map(str::to_string),
        variant: variant.map(str::to_string),
        ..Default::default()
    }
}

#[test]
fn platform_name_os_and_architecture_returns_platform_string() {
    let named = platform_name(&setup_oci_platform(Some("linux"), Some("arm64"), Some("v8")));

    assert_eq!(named.as_deref(), Some("linux/arm64"));
}

#[test]
fn platform_name_missing_field_returns_none() {
    assert_eq!(platform_name(&setup_oci_platform(Some("linux"), None, None)), None);
    assert_eq!(platform_name(&setup_oci_platform(None, Some("amd64"), None)), None);
}

#[tokio::test]
async fn resolve_platform_with_amd64_host_skips_registry_lookup() {
    let runtime_client = setup_fake_client(Ok(vec!["linux/amd64"]));

    let resolved =
        resolve_platform_with("skips-lookup:tag", "linux/amd64", &runtime_client).await;

    assert_eq!(resolved, "linux/amd64");
    assert_eq!(runtime_client.lookups.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn resolve_platform_with_amd64_only_image_returns_fallback_platform() {
    let runtime_client = setup_fake_client(Ok(vec!["linux/amd64"]));

    let resolved = resolve_platform_with("amd64-only-image:tag", "linux/arm64", &runtime_client).await;

    assert_eq!(resolved, "linux/amd64");
    assert_eq!(runtime_client.lookups.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn published_platform_names_mixed_entries_keeps_named_platforms() {
    let platforms = vec![
        setup_oci_platform(Some("linux"), Some("amd64"), None),
        setup_oci_platform(Some("unknown"), Some("unknown"), None),
        setup_oci_platform(Some("linux"), Some("arm64"), Some("v8")),
        setup_oci_platform(Some("linux"), None, None),
    ];

    let named = published_platform_names(&platforms);

    assert_eq!(named, vec!["linux/amd64", "unknown/unknown", "linux/arm64"]);
}

#[tokio::test]
async fn resolve_platform_with_repeated_reference_looks_up_once() {
    let runtime_client = setup_fake_client(Ok(vec!["linux/amd64"]));

    let first = resolve_platform_with("cached-image:tag", "linux/arm64", &runtime_client).await;
    let second = resolve_platform_with("cached-image:tag", "linux/arm64", &runtime_client).await;

    assert_eq!(first, "linux/amd64");
    assert_eq!(second, "linux/amd64");
    assert_eq!(runtime_client.lookups.load(Ordering::SeqCst), 1);
}
