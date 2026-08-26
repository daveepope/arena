use arena::dependency::RunnableDependency;
use arena_kafka::{KafkaDependency, KafkaFlavor, KafkaImpl};
use async_trait::async_trait;

struct NoopKafkaImpl;

#[async_trait]
impl KafkaImpl for NoopKafkaImpl {
    async fn start(&mut self, _port: u16, _image_name: &str, _image_tag: &str, _container_name: &str) {}

    async fn stop(&mut self) {}

    fn bootstrap_servers(&self) -> Option<&str> {
        None
    }
}

#[test]
fn build_default_flavor_produces_identifier_with_prefix() {
    let dep = KafkaDependency::builder("builder-default")
        .with_impl(NoopKafkaImpl)
        .build();

    assert!(dep.identifier().contains("builder-default"));
}

#[test]
fn build_apache_native_flavor_uses_default_kafka_impl() {
    let dep = KafkaDependency::builder("builder-apache")
        .with_flavor(KafkaFlavor::ApacheNative)
        .build();

    assert!(dep.identifier().contains("builder-apache"));
    assert!(dep.bootstrap_servers().is_none());
}

#[test]
fn build_confluent_flavor_uses_default_kafka_impl() {
    let dep = KafkaDependency::builder("builder-confluent")
        .with_flavor(KafkaFlavor::Confluent)
        .build();

    assert!(dep.identifier().contains("builder-confluent"));
    assert!(dep.bootstrap_servers().is_none());
}

#[test]
fn build_confluent_flavor_with_network_uses_default_kafka_impl() {
    let dep = KafkaDependency::builder("builder-confluent-net")
        .with_flavor(KafkaFlavor::Confluent)
        .with_network("builder-test-network")
        .build();

    assert!(dep.identifier().contains("builder-confluent-net"));
}

#[test]
fn build_apache_flavor_with_network_uses_default_kafka_impl() {
    let dep = KafkaDependency::builder("builder-apache-net")
        .with_flavor(KafkaFlavor::ApacheNative)
        .with_network("builder-test-network")
        .build();

    assert!(dep.identifier().contains("builder-apache-net"));
}

#[test]
fn with_topic_registers_topic_for_creation() {
    let dep = KafkaDependency::builder("builder-topics")
        .with_impl(NoopKafkaImpl)
        .with_topic("topic-a")
        .with_topic("topic-b")
        .build();

    assert!(dep.identifier().contains("builder-topics"));
}

#[test]
fn with_image_sets_image_tag() {
    let dep = KafkaDependency::builder("builder-image")
        .with_impl(NoopKafkaImpl)
        .with_image("custom-tag")
        .build();

    assert!(dep.identifier().contains("builder-image"));
}

#[test]
fn with_container_tag_sets_image_tag() {
    let dep = KafkaDependency::builder("builder-container-tag")
        .with_impl(NoopKafkaImpl)
        .with_container_tag("custom-tag")
        .build();

    assert!(dep.identifier().contains("builder-container-tag"));
}

#[test]
fn with_container_name_and_image_name_set_without_panic() {
    let dep = KafkaDependency::builder("builder-names")
        .with_impl(NoopKafkaImpl)
        .with_image_name("custom-image")
        .with_image_tag("custom-tag")
        .with_container_name("custom-container")
        .with_port(4242)
        .build();

    assert!(dep.identifier().contains("builder-names"));
}

#[test]
fn with_child_dependencies_accepted_without_panic() {
    let dep = KafkaDependency::builder("builder-children")
        .with_impl(NoopKafkaImpl)
        .with_child_dependencies(Vec::new())
        .build();

    assert!(dep.children().is_empty());
}
