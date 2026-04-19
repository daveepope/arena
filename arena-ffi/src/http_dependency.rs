pub(crate) mod on_dependency_startup;

use arena::Dependency;
use arena_http::HttpDependency;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct HttpDependencyConfig {
    pub identifier: String,
    #[serde(default)]
    pub image_name: Option<String>,
    #[serde(default)]
    pub image_tag: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub container_name: Option<String>,
}

pub(crate) fn build(
    config: &HttpDependencyConfig,
    network: &str,
) -> Result<Dependency, String> {
    let mut builder =
        HttpDependency::builder(&config.identifier).with_port(config.port.unwrap_or(0));
    if let Some(ref container_name) = config.container_name {
        builder = builder.with_container_name(container_name);
    }
    builder = builder.with_network(network);
    if let Some(ref image_name) = config.image_name {
        builder = builder.with_image_name(image_name);
    }
    if let Some(ref image_tag) = config.image_tag {
        builder = builder.with_image_tag(image_tag);
    }
    Ok(Box::new(builder.build()))
}
