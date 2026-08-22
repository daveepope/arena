use arena::healthcheck::ReadinessCheck;
use arena_containerized_component::builder::ContainerizedComponentBuilder;
use arena_containerized_component::containerized_component::ContainerizedComponent;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct FakeReadinessCheck;

#[async_trait::async_trait]
impl ReadinessCheck for FakeReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _target: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn with_volume_mapping_single_mapping_chains_for_further_building() {
    let _builder = ContainerizedComponent::builder("probe", "FROM alpine:3.20")
        .with_volume_mapping("/host/path", "/container/path");
}

#[test]
fn with_volume_mapping_multiple_mappings_chain_for_further_building() {
    let _builder = ContainerizedComponent::builder("probe", "FROM alpine:3.20")
        .with_volume_mapping("/host/one", "/container/one")
        .with_volume_mapping("/host/two", "/container/two");
}

#[test]
fn from_image_chains_for_further_building() {
    let _builder = ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_port_mapping(8080, 80);
}

#[test]
fn with_platform_chains_for_further_building() {
    let _builder = ContainerizedComponent::builder("probe", "FROM alpine:3.20")
        .with_platform("linux/arm64");
}

#[test]
fn from_image_with_platform_chains_for_further_building() {
    let _builder = ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_platform("linux/amd64");
}

#[test]
fn with_network_chains_for_further_building() {
    let _builder =
        ContainerizedComponent::from_image("probe", "alpine:3.20").with_network("probe-net");
}

#[test]
fn with_network_alias_chains_for_further_building() {
    let _builder = ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_network("probe-net")
        .with_network_alias("probe-alias");
}

#[test]
fn with_env_var_multiple_chains_for_further_building() {
    let _builder = ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_env_var("KEY_ONE", "value-one")
        .with_env_var("KEY_TWO", "value-two");
}

#[test]
fn with_runtime_arg_chains_for_further_building() {
    let _builder = ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_runtime_arg("some_arg", "some_value");
}

#[test]
fn with_host_mapping_chains_for_further_building() {
    let _builder = ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_host_mapping("host.docker.internal:host-gateway");
}

#[test]
fn with_child_components_chains_for_further_building() {
    let _builder =
        ContainerizedComponent::from_image("probe", "alpine:3.20").with_child_components(vec![]);
}

#[test]
fn with_readiness_check_chains_for_further_building() {
    let _builder = ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_readiness_check(FakeReadinessCheck, "http://localhost:8080/health");
}

#[test]
fn with_readiness_check_timeout_chains_for_further_building() {
    let _builder = ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_readiness_check_timeout(FakeReadinessCheck, "http://localhost:8080/health", 5_000);
}

#[tokio::test]
async fn from_image_with_build_context_build_returns_invalid_configuration_error() {
    let err = ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_build_context(".")
        .build()
        .await
        .err()
        .expect("build should reject with_build_context combined with from_image");

    assert!(err
        .to_string()
        .contains("with_build_context has no effect when using from_image"));
}

#[tokio::test]
async fn from_image_with_image_tag_build_returns_invalid_configuration_error() {
    let err = ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_image_tag("custom-tag")
        .build()
        .await
        .err()
        .expect("build should reject with_image_tag combined with from_image");

    assert!(err
        .to_string()
        .contains("with_image_tag has no effect when using from_image"));
}

struct TempContextDir(PathBuf);

impl TempContextDir {
    fn create(name: &str) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{name}-{ts}"));
        std::fs::create_dir_all(&dir).expect("create temp build context dir");
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempContextDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tar_file_entries(bytes: &[u8]) -> Vec<(String, String)> {
    let mut archive = tar::Archive::new(bytes);
    let mut entries: Vec<(String, String)> = archive
        .entries()
        .expect("read tar entries")
        .map(|entry| {
            let mut entry = entry.expect("read tar entry");
            let is_file = entry.header().entry_type().is_file();
            let path = entry
                .path()
                .expect("entry path")
                .to_string_lossy()
                .into_owned();
            let mut content = String::new();
            if is_file {
                entry.read_to_string(&mut content).expect("read entry content");
            }
            (path, content, is_file)
        })
        .filter(|(_, _, is_file)| *is_file)
        .map(|(path, content, _)| (path, content))
        .collect();
    entries.sort();
    entries
}

#[test]
fn create_build_context_tar_no_build_context_includes_only_containerfile() {
    let bytes =
        ContainerizedComponentBuilder::create_build_context_tar("probe", "FROM alpine:3.20", &None);

    assert_eq!(
        tar_file_entries(&bytes),
        vec![(".arena.Dockerfile".to_string(), "FROM alpine:3.20".to_string())]
    );
}

#[test]
fn create_build_context_tar_with_build_context_includes_nested_files_skips_hidden_and_skip_dirs() {
    let context = TempContextDir::create("arena-build-context-test");
    std::fs::write(context.path().join("keep.txt"), "keep").expect("write keep.txt");
    std::fs::write(context.path().join(".hidden"), "secret").expect("write .hidden");
    let skipped = context.path().join("target");
    std::fs::create_dir_all(&skipped).expect("create target dir");
    std::fs::write(skipped.join("build_artifact.txt"), "artifact").expect("write build artifact");
    let nested = context.path().join("nested");
    std::fs::create_dir_all(&nested).expect("create nested dir");
    std::fs::write(nested.join("inner.txt"), "inner").expect("write inner.txt");

    let bytes = ContainerizedComponentBuilder::create_build_context_tar(
        "probe",
        "FROM alpine:3.20",
        &Some(context.path().to_path_buf()),
    );

    assert_eq!(
        tar_file_entries(&bytes),
        vec![
            (".arena.Dockerfile".to_string(), "FROM alpine:3.20".to_string()),
            ("keep.txt".to_string(), "keep".to_string()),
            ("nested/inner.txt".to_string(), "inner".to_string()),
        ]
    );
}

#[test]
fn resolve_path_absolute_path_returns_unchanged() {
    let absolute = std::env::current_dir()
        .expect("current dir")
        .join("Cargo.toml");

    assert_eq!(
        ContainerizedComponentBuilder::resolve_path(absolute.clone()),
        absolute
    );
}

#[test]
fn resolve_path_relative_dot_resolves_via_ancestor_search() {
    let expected = std::env::current_dir().expect("current dir").join(".");

    assert_eq!(
        ContainerizedComponentBuilder::resolve_path(PathBuf::from(".")),
        expected
    );
}

#[test]
fn resolve_path_missing_relative_path_falls_back_to_current_dir_join() {
    let missing = PathBuf::from("arena-nonexistent-context-8f3c1e/definitely-missing");
    let expected = std::env::current_dir().expect("current dir").join(&missing);

    assert_eq!(
        ContainerizedComponentBuilder::resolve_path(missing),
        expected
    );
}
