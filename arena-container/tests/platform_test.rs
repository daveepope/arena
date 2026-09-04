use arena_container::platform::{
    docker_platform, platform_for_arch, platform_name, published_platform_names,
    resolve_platform_with, select_platform, ImagePlatformClient,
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
    local: Option<String>,
    lookups: AtomicUsize,
}

struct FailThenPublishClient {
    published: Vec<String>,
    lookups: AtomicUsize,
}

fn setup_fail_then_publish_client(published: Vec<&str>) -> FailThenPublishClient {
    FailThenPublishClient {
        published: published.into_iter().map(str::to_string).collect(),
        lookups: AtomicUsize::new(0),
    }
}

impl ImagePlatformClient for FailThenPublishClient {
    async fn published_platforms(&self, _image_reference: &str) -> Result<Vec<String>, String> {
        if self.lookups.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err("registry unreachable".to_string());
        }
        Ok(self.published.clone())
    }

    async fn local_platform(&self, _image_reference: &str) -> Option<String> {
        None
    }
}

fn setup_fake_client(published: Result<Vec<&str>, &str>) -> FakePublishedPlatformClient {
    setup_fake_client_with_local(published, None)
}

fn setup_fake_client_with_local(
    published: Result<Vec<&str>, &str>,
    local: Option<&str>,
) -> FakePublishedPlatformClient {
    FakePublishedPlatformClient {
        published: published
            .map(|platforms| platforms.into_iter().map(str::to_string).collect())
            .map_err(str::to_string),
        local: local.map(str::to_string),
        lookups: AtomicUsize::new(0),
    }
}

impl ImagePlatformClient for FakePublishedPlatformClient {
    async fn published_platforms(&self, _image_reference: &str) -> Result<Vec<String>, String> {
        self.lookups.fetch_add(1, Ordering::SeqCst);
        self.published.clone()
    }

    async fn local_platform(&self, _image_reference: &str) -> Option<String> {
        self.local.clone()
    }
}

#[tokio::test]
async fn select_platform_published_variants_returns_runnable_platform() {
    let cases: Vec<(Result<Vec<&str>, &str>, &str, Option<&str>)> = vec![
        (Ok(vec!["linux/amd64", "linux/arm64"]), "linux/arm64", Some("linux/arm64")),
        (Ok(vec!["linux/amd64", "unknown/unknown"]), "linux/arm64", Some("linux/amd64")),
        (Ok(vec!["linux/arm/v7"]), "linux/arm64", Some("linux/arm64")),
        (Ok(vec![]), "linux/arm64", Some("linux/arm64")),
        (Err("registry unreachable"), "linux/arm64", None),
        (Ok(vec!["linux/amd64"]), "linux/amd64", Some("linux/amd64")),
    ];

    for (published, host_platform, expected) in cases {
        let runtime_client = setup_fake_client(published.clone());

        let selected = select_platform("image:tag", host_platform, &runtime_client).await;

        assert_eq!(
            selected.as_deref(),
            expected,
            "published: {published:?}, host: {host_platform}"
        );
    }
}

#[tokio::test]
async fn select_platform_unreachable_registry_returns_local_image_platform() {
    let runtime_client =
        setup_fake_client_with_local(Err("registry unreachable"), Some("linux/amd64"));

    let selected = select_platform("local-only:tag", "linux/arm64", &runtime_client).await;

    assert_eq!(selected.as_deref(), Some("linux/amd64"));
}

#[tokio::test]
async fn resolve_platform_with_unreachable_registry_and_local_image_caches_local_platform() {
    let runtime_client =
        setup_fake_client_with_local(Err("registry unreachable"), Some("linux/amd64"));

    let first = resolve_platform_with("local-cached:tag", "linux/arm64", &runtime_client).await;
    let second = resolve_platform_with("local-cached:tag", "linux/arm64", &runtime_client).await;

    assert_eq!(first, "linux/amd64");
    assert_eq!(second, "linux/amd64");
    assert_eq!(runtime_client.lookups.load(Ordering::SeqCst), 1);
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

#[test]
fn platform_for_arch_known_and_unknown_arches_map_to_linux_platforms() {
    let cases = [
        ("x86_64", "linux/amd64"),
        ("aarch64", "linux/arm64"),
        ("powerpc64", "linux/powerpc64"),
    ];

    for (arch, expected) in cases {
        assert_eq!(platform_for_arch(arch), expected, "arch: {arch}");
    }
}

#[tokio::test]
async fn resolve_platform_with_failed_lookup_retries_on_the_next_call() {
    let runtime_client = setup_fail_then_publish_client(vec!["linux/amd64"]);

    let first =
        resolve_platform_with("transient-failure:tag", "linux/arm64", &runtime_client).await;
    let second =
        resolve_platform_with("transient-failure:tag", "linux/arm64", &runtime_client).await;

    assert_eq!(first, "linux/arm64");
    assert_eq!(second, "linux/amd64");
    assert_eq!(runtime_client.lookups.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn resolve_platform_with_second_host_platform_resolves_for_that_host() {
    let runtime_client = setup_fake_client(Ok(vec!["linux/amd64"]));

    let arm64 = resolve_platform_with("two-hosts:tag", "linux/arm64", &runtime_client).await;
    let riscv64 = resolve_platform_with("two-hosts:tag", "linux/riscv64", &runtime_client).await;

    assert_eq!(arm64, "linux/amd64");
    assert_eq!(riscv64, "linux/amd64");
    assert_eq!(runtime_client.lookups.load(Ordering::SeqCst), 2);
}
