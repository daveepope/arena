use bollard::models::{Mount, MountTmpfsOptions, MountTypeEnum};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MountType {
    Bind,
    Volume,
    Tmpfs,
}

pub struct MountSpec {
    pub mount_type: MountType,
    pub source: Option<String>,
    pub container_path: String,
    pub read_only: bool,
    pub tmpfs_size_bytes: Option<i64>,
}

pub fn to_docker_mount(mount: &MountSpec) -> Mount {
    Mount {
        target: Some(mount.container_path.clone()),
        source: mount.source.clone(),
        typ: Some(match mount.mount_type {
            MountType::Bind => MountTypeEnum::BIND,
            MountType::Volume => MountTypeEnum::VOLUME,
            MountType::Tmpfs => MountTypeEnum::TMPFS,
        }),
        read_only: Some(mount.read_only),
        tmpfs_options: mount.tmpfs_size_bytes.map(|size_bytes| MountTmpfsOptions {
            size_bytes: Some(size_bytes),
            ..Default::default()
        }),
        ..Default::default()
    }
}
