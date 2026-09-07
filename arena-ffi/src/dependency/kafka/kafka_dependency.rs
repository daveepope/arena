use arena::Dependency;
use arena_kafka::{KafkaDependency, KafkaFlavor};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct KafkaDependencyConfig {
    pub identifier: String,
    #[serde(default)]
    pub expiry_seconds: Option<u64>,
    #[serde(default)]
    pub image_name: Option<String>,
    #[serde(default)]
    pub flavor: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub container_name: Option<String>,
    #[serde(default)]
    pub topics: Option<Vec<String>>,
}

pub(crate) fn build(config: &KafkaDependencyConfig, network: Option<&str>) -> Result<Dependency, String> {
    let flavor = match config.flavor.as_deref() {
        Some("confluent") => KafkaFlavor::Confluent,
        Some("apache_native") | None => KafkaFlavor::ApacheNative,
        Some(other) => return Err(format!("unknown kafka flavor '{other}'")),
    };
    let mut builder = KafkaDependency::builder(&config.identifier)
        .with_flavor(flavor)
        .with_port(config.port.unwrap_or(9092));
    if let Some(network) = network {
        builder = builder.with_network(network);
    }
    if let Some(ref container_name) = config.container_name {
        builder = builder.with_container_name(container_name);
    }
    if let Some(ref image_name) = config.image_name {
        builder = builder.with_image_name(image_name);
    }
    for topic in config.topics.as_deref().unwrap_or(&[]) {
        builder = builder.with_topic(topic);
    }
    match crate::dependency::expiry::expiry_override(config.expiry_seconds) {
        Some(crate::dependency::expiry::ExpiryOverride::Disabled) => {
            builder = builder.without_expiry();
        }
        Some(crate::dependency::expiry::ExpiryOverride::After(expiry)) => {
            builder = builder.with_expiry(expiry);
        }
        None => {}
    }

    Ok(Box::new(builder.build()))
}
