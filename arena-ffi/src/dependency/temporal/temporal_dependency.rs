use arena::Dependency;
use arena_temporal::TemporalDependency;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TemporalDependencyConfig {
    pub identifier: String,
    #[serde(default)]
    pub image_name: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub ui_port: Option<u16>,
    #[serde(default)]
    pub container_name: Option<String>,
}

pub fn build(config: &TemporalDependencyConfig, network: Option<&str>) -> Result<Dependency, String> {
    let mut builder = TemporalDependency::builder(&config.identifier);
    if let Some(port) = config.port {
        builder = builder.with_port(port);
    }
    if let Some(ui_port) = config.ui_port {
        builder = builder.with_ui_port(ui_port);
    }
    if let Some(image) = config.image.as_deref() {
        builder = builder.with_image(image);
    }
    if let Some(ref image_name) = config.image_name {
        builder = builder.with_image_name(image_name);
    }
    if let Some(network) = network {
        builder = builder.with_network(network);
    }
    if let Some(ref container_name) = config.container_name {
        builder = builder.with_container_name(container_name);
    }
    Ok(Box::new(builder.build()))
}
