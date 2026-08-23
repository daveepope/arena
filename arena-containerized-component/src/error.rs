use std::fmt;

#[derive(Debug)]
pub enum ContainerizedComponentBuildError {
    InvalidConfiguration(String),
    RuntimeUnavailable(String),
    ImagePull(String),
    ImageBuild { identifier: String, message: String },
}

impl fmt::Display for ContainerizedComponentBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerizedComponentBuildError::InvalidConfiguration(message) => {
                write!(f, "{message}")
            }
            ContainerizedComponentBuildError::RuntimeUnavailable(message) => {
                write!(f, "{message}")
            }
            ContainerizedComponentBuildError::ImagePull(message) => write!(f, "{message}"),
            ContainerizedComponentBuildError::ImageBuild { identifier, message } => {
                write!(f, "{identifier}: image build failed: {message}")
            }
        }
    }
}

impl std::error::Error for ContainerizedComponentBuildError {}

impl From<arena_container::image::ImagePullError> for ContainerizedComponentBuildError {
    fn from(err: arena_container::image::ImagePullError) -> Self {
        ContainerizedComponentBuildError::ImagePull(err.to_string())
    }
}
