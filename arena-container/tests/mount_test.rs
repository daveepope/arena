use arena_container::mount::{to_docker_mount, MountSpec, MountType};
use bollard::models::MountTypeEnum;

#[test]
fn to_docker_mount_bind_maps_type_and_fields() {
    let spec = MountSpec {
        mount_type: MountType::Bind,
        source: Some("/host/data".to_string()),
        container_path: "/mnt/data".to_string(),
        read_only: true,
        tmpfs_size_bytes: None,
    };

    let mount = to_docker_mount(&spec);

    assert_eq!(mount.typ, Some(MountTypeEnum::BIND));
    assert_eq!(mount.source.as_deref(), Some("/host/data"));
    assert_eq!(mount.target.as_deref(), Some("/mnt/data"));
    assert_eq!(mount.read_only, Some(true));
    assert!(mount.tmpfs_options.is_none());
}

#[test]
fn to_docker_mount_volume_maps_type() {
    let spec = MountSpec {
        mount_type: MountType::Volume,
        source: Some("my-volume".to_string()),
        container_path: "/mnt/data".to_string(),
        read_only: false,
        tmpfs_size_bytes: None,
    };

    let mount = to_docker_mount(&spec);

    assert_eq!(mount.typ, Some(MountTypeEnum::VOLUME));
    assert_eq!(mount.source.as_deref(), Some("my-volume"));
}

#[test]
fn to_docker_mount_tmpfs_sets_size_bytes() {
    let spec = MountSpec {
        mount_type: MountType::Tmpfs,
        source: None,
        container_path: "/mnt/scratch".to_string(),
        read_only: false,
        tmpfs_size_bytes: Some(1024),
    };

    let mount = to_docker_mount(&spec);

    assert_eq!(mount.typ, Some(MountTypeEnum::TMPFS));
    assert!(mount.source.is_none());
    assert_eq!(
        mount.tmpfs_options.and_then(|o| o.size_bytes),
        Some(1024)
    );
}

#[test]
fn to_docker_mount_tmpfs_without_size_omits_tmpfs_options_size() {
    let spec = MountSpec {
        mount_type: MountType::Tmpfs,
        source: None,
        container_path: "/mnt/scratch".to_string(),
        read_only: false,
        tmpfs_size_bytes: None,
    };

    let mount = to_docker_mount(&spec);

    assert!(mount.tmpfs_options.is_none());
}
