use arena_container::image::{
    image_matches_platform, pull_image, pull_status_line, registry_host,
    resolve_registry_credentials, to_docker_credentials, ImagePullClient,
};
use base64::Engine;
use bollard::auth::DockerCredentials;
use bollard::models::ImageInspect;
use docker_credential::DockerCredential;
use futures::stream::{self, BoxStream, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static DOCKER_ENV_LOCK: Mutex<()> = Mutex::new(());

struct FakeImagePullClient {
    events: Vec<Result<String, String>>,
    present_locally: bool,
    pull_invoked: AtomicBool,
}

fn setup_fake_client(events: Vec<Result<String, String>>, present_locally: bool) -> FakeImagePullClient {
    FakeImagePullClient {
        events,
        present_locally,
        pull_invoked: AtomicBool::new(false),
    }
}

impl ImagePullClient for FakeImagePullClient {
    fn pull(
        &self,
        _image: &str,
        _platform: &str,
        _credentials: Option<DockerCredentials>,
    ) -> BoxStream<'_, Result<String, String>> {
        self.pull_invoked.store(true, Ordering::SeqCst);
        stream::iter(self.events.clone()).boxed()
    }

    async fn image_present_locally(&self, _image: &str, _platform: &str) -> bool {
        self.present_locally
    }
}

fn setup_inspect(os: Option<&str>, architecture: Option<&str>, variant: Option<&str>) -> ImageInspect {
    ImageInspect {
        os: os.map(str::to_string),
        architecture: architecture.map(str::to_string),
        variant: variant.map(str::to_string),
        ..Default::default()
    }
}

#[tokio::test]
async fn pull_image_ok_events_completes_without_error() {
    let client = setup_fake_client(
        vec![Ok("Pulling fs layer".to_string()), Ok(String::new())],
        false,
    );

    pull_image("web", "redis:8-alpine", "linux/amd64", &client)
        .await
        .expect("pull should succeed");
}

#[tokio::test]
async fn pull_image_error_event_returns_error_with_identifier_and_message() {
    let client = setup_fake_client(vec![Err("daemon unreachable".to_string())], false);

    let err = pull_image("web", "redis:8-alpine", "linux/amd64", &client)
        .await
        .expect_err("pull should fail");

    assert_eq!(
        err.to_string(),
        "web: image pull failed: daemon unreachable"
    );
}

#[tokio::test]
async fn pull_image_present_locally_skips_pull() {
    let client = setup_fake_client(Vec::new(), true);

    pull_image("web", "redis:8-alpine", "linux/amd64", &client)
        .await
        .expect("pull should succeed");

    assert!(
        !client.pull_invoked.load(Ordering::SeqCst),
        "expected pull() not to be invoked when the image is already present locally"
    );
}

#[test]
fn pull_status_line_none_returns_none() {
    assert_eq!(pull_status_line(None), None);
}

#[test]
fn pull_status_line_blank_returns_none() {
    assert_eq!(pull_status_line(Some("   ")), None);
}

#[test]
fn pull_status_line_trailing_whitespace_returns_trimmed() {
    assert_eq!(pull_status_line(Some("Pulling fs layer\n")), Some("Pulling fs layer"));
}

#[test]
fn image_matches_platform_same_os_and_arch_returns_true() {
    let inspect = setup_inspect(Some("linux"), Some("amd64"), None);
    assert!(image_matches_platform(&inspect, "linux/amd64"));
}

#[test]
fn image_matches_platform_different_arch_returns_false() {
    let inspect = setup_inspect(Some("linux"), Some("amd64"), None);
    assert!(!image_matches_platform(&inspect, "linux/arm64"));
}

#[test]
fn image_matches_platform_different_os_returns_false() {
    let inspect = setup_inspect(Some("windows"), Some("amd64"), None);
    assert!(!image_matches_platform(&inspect, "linux/amd64"));
}

#[test]
fn image_matches_platform_missing_inspect_fields_returns_false() {
    let inspect = setup_inspect(None, None, None);
    assert!(!image_matches_platform(&inspect, "linux/amd64"));
}

#[test]
fn image_matches_platform_matching_variant_returns_true() {
    let inspect = setup_inspect(Some("linux"), Some("arm"), Some("v7"));
    assert!(image_matches_platform(&inspect, "linux/arm/v7"));
}

#[test]
fn image_matches_platform_mismatched_variant_returns_false() {
    let inspect = setup_inspect(Some("linux"), Some("arm"), Some("v6"));
    assert!(!image_matches_platform(&inspect, "linux/arm/v7"));
}

#[test]
fn registry_host_dotted_private_registry_returns_host() {
    assert_eq!(
        registry_host("123456789012.dkr.ecr.us-east-1.amazonaws.com/my-repo:latest"),
        "123456789012.dkr.ecr.us-east-1.amazonaws.com"
    );
}

#[test]
fn registry_host_host_with_port_returns_host() {
    assert_eq!(
        registry_host("localhost:5000/my-image:latest"),
        "localhost:5000"
    );
}

#[test]
fn registry_host_localhost_without_port_returns_localhost() {
    assert_eq!(registry_host("localhost/my-image:latest"), "localhost");
}

#[test]
fn registry_host_bare_image_returns_docker_hub() {
    assert_eq!(
        registry_host("redis:8-alpine"),
        "https://index.docker.io/v1/"
    );
}

#[test]
fn registry_host_hub_namespaced_image_returns_docker_hub() {
    assert_eq!(
        registry_host("library/postgres:15"),
        "https://index.docker.io/v1/"
    );
}

#[test]
fn registry_host_third_party_registry_returns_host() {
    assert_eq!(registry_host("ghcr.io/org/repo:tag"), "ghcr.io");
}

#[tokio::test]
async fn resolve_registry_credentials_configured_auth_returns_credentials() {
    let _guard = DOCKER_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!(
        "arena-container-docker-config-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create temp DOCKER_CONFIG dir");
    let host = "registry.example.com";
    let encoded_auth = base64::engine::general_purpose::STANDARD.encode("resolved-user:resolved-pass");
    std::fs::write(
        dir.join("config.json"),
        format!(
            r#"{{"auths": {{"{host}": {{"auth": "{encoded_auth}"}}}}}}"#,
        ),
    )
    .expect("write config.json");

    let prior = std::env::var("DOCKER_CONFIG").ok();
    unsafe {
        std::env::set_var("DOCKER_CONFIG", &dir);
    }

    let credentials = resolve_registry_credentials(&format!("{host}/my-repo:latest")).await;

    unsafe {
        match &prior {
            Some(value) => std::env::set_var("DOCKER_CONFIG", value),
            None => std::env::remove_var("DOCKER_CONFIG"),
        }
    }
    let _ = std::fs::remove_dir_all(&dir);

    let credentials = credentials.expect("credentials should resolve from configured auth");
    assert_eq!(credentials.username.as_deref(), Some("resolved-user"));
    assert_eq!(credentials.password.as_deref(), Some("resolved-pass"));
    assert_eq!(credentials.serveraddress.as_deref(), Some(host));
}

#[tokio::test]
async fn resolve_registry_credentials_ecr_style_cred_helper_returns_credentials() {
    let _guard = DOCKER_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let suffix = std::process::id();
    let helper_dir = std::env::temp_dir().join(format!("arena-container-cred-helper-{suffix}"));
    std::fs::create_dir_all(&helper_dir).expect("create temp cred helper dir");
    let helper_path = helper_dir.join("docker-credential-arena-fake-ecr");
    std::fs::write(
        &helper_path,
        "#!/bin/sh\ncat > /dev/null\necho '{\"Username\":\"AWS\",\"Secret\":\"ecr-fake-token\"}'\n",
    )
    .expect("write fake credential helper");
    let mut perms = std::fs::metadata(&helper_path)
        .expect("read helper metadata")
        .permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&helper_path, perms).expect("make helper executable");

    let config_dir = std::env::temp_dir().join(format!("arena-container-docker-config-ecr-{suffix}"));
    std::fs::create_dir_all(&config_dir).expect("create temp DOCKER_CONFIG dir");
    let host = "123456789012.dkr.ecr.us-east-1.amazonaws.com";
    std::fs::write(
        config_dir.join("config.json"),
        format!(r#"{{"credHelpers": {{"{host}": "arena-fake-ecr"}}}}"#),
    )
    .expect("write config.json");

    let prior_path = std::env::var("PATH").ok();
    let prior_docker_config = std::env::var("DOCKER_CONFIG").ok();
    unsafe {
        let new_path = match &prior_path {
            Some(existing) => format!("{}:{existing}", helper_dir.display()),
            None => helper_dir.display().to_string(),
        };
        std::env::set_var("PATH", new_path);
        std::env::set_var("DOCKER_CONFIG", &config_dir);
    }

    let credentials =
        resolve_registry_credentials(&format!("{host}/my-repo:latest")).await;

    unsafe {
        match &prior_path {
            Some(value) => std::env::set_var("PATH", value),
            None => std::env::remove_var("PATH"),
        }
        match &prior_docker_config {
            Some(value) => std::env::set_var("DOCKER_CONFIG", value),
            None => std::env::remove_var("DOCKER_CONFIG"),
        }
    }
    let _ = std::fs::remove_dir_all(&helper_dir);
    let _ = std::fs::remove_dir_all(&config_dir);

    let credentials = credentials.expect("credentials should resolve via the ECR-style cred helper");
    assert_eq!(credentials.username.as_deref(), Some("AWS"));
    assert_eq!(credentials.password.as_deref(), Some("ecr-fake-token"));
    assert_eq!(credentials.serveraddress.as_deref(), Some(host));
}

#[test]
fn to_docker_credentials_username_password_maps_fields() {
    let credentials = to_docker_credentials(
        DockerCredential::UsernamePassword("user".to_string(), "pass".to_string()),
        "ghcr.io",
    );

    assert_eq!(credentials.username.as_deref(), Some("user"));
    assert_eq!(credentials.password.as_deref(), Some("pass"));
    assert_eq!(credentials.serveraddress.as_deref(), Some("ghcr.io"));
    assert_eq!(credentials.identitytoken, None);
}

#[test]
fn to_docker_credentials_identity_token_maps_fields() {
    let credentials = to_docker_credentials(
        DockerCredential::IdentityToken("token-value".to_string()),
        "ghcr.io",
    );

    assert_eq!(credentials.identitytoken.as_deref(), Some("token-value"));
    assert_eq!(credentials.serveraddress.as_deref(), Some("ghcr.io"));
    assert_eq!(credentials.username, None);
    assert_eq!(credentials.password, None);
}
