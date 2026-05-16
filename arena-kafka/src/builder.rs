use crate::kafka_dependency::container_impl::{ConfluentKafkaContainerImpl, KafkaContainerImpl};
use crate::kafka_dependency::{KafkaDependency, KafkaImpl};
use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KafkaFlavor {
    ApacheNative,
    Confluent,
}

pub struct KafkaDependencyBuilder {
    identifier: String,
    kafka_impl: Option<Box<dyn KafkaImpl>>,
    flavor: KafkaFlavor,
    port: Option<u16>,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    image_name: Option<String>,
    image_tag: Option<String>,
    container_name: Option<String>,
    network: Option<String>,
    readiness_check: Option<Box<dyn ReadinessCheck>>,
    topics: Vec<String>,
}

impl KafkaDependencyBuilder {
    const APACHE_DEFAULT_PORT: u16 = 9092;
    const APACHE_DEFAULT_IMAGE_NAME: &'static str = "apache/kafka";
    const APACHE_DEFAULT_TAG: &'static str = "3.8.0";

    const CONFLUENT_DEFAULT_PORT: u16 = 9093;
    const CONFLUENT_DEFAULT_IMAGE_NAME: &'static str = "confluentinc/cp-kafka";
    const CONFLUENT_DEFAULT_TAG: &'static str = "6.1.1";

    pub(crate) fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            kafka_impl: None,
            flavor: KafkaFlavor::ApacheNative,
            port: None,
            dependencies: None,
            image_name: None,
            image_tag: None,
            container_name: None,
            network: None,
            readiness_check: None,
            topics: Vec::new(),
        }
    }

    pub fn with_topic(mut self, topic: impl Into<String>) -> Self {
        self.topics.push(topic.into());
        self
    }

    pub fn with_impl<W>(mut self, wrapper: W) -> Self
    where
        W: KafkaImpl + 'static,
    {
        self.kafka_impl = Some(Box::new(wrapper));
        self
    }

    pub fn with_flavor(mut self, flavor: KafkaFlavor) -> Self {
        self.flavor = flavor;
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Option::from(port);
        self
    }

    pub fn with_child_dependencies(
        mut self,
        dependencies: Vec<Box<dyn RunnableDependency>>,
    ) -> Self {
        self.dependencies = Option::from(dependencies);
        self
    }

    pub fn with_image_name(mut self, image_name: impl Into<String>) -> Self {
        self.image_name = Some(image_name.into());
        self
    }

    pub fn with_image_tag(mut self, image_tag: impl Into<String>) -> Self {
        self.image_tag = Some(image_tag.into());
        self
    }

    pub fn with_image(self, image_tag: impl Into<String>) -> Self {
        self.with_image_tag(image_tag)
    }

    pub fn with_readiness_check<W>(mut self, check: W) -> Self
    where
        W: ReadinessCheck + 'static,
    {
        self.readiness_check = Some(Box::new(check));
        self
    }

    pub fn with_container_tag(self, image_tag: impl Into<String>) -> Self {
        self.with_image_tag(image_tag)
    }

    pub fn with_container_name(mut self, container_name: impl Into<String>) -> Self {
        self.container_name = Some(container_name.into());
        self
    }

    pub fn with_network(mut self, network: impl Into<String>) -> Self {
        self.network = Some(network.into());
        self
    }

    pub fn build(self) -> KafkaDependency {
        let KafkaDependencyBuilder {
            identifier,
            kafka_impl,
            flavor,
            port,
            dependencies,
            image_name,
            image_tag,
            container_name,
            network,
            readiness_check,
            topics,
        } = self;

        let (default_port, default_image_name, default_tag) = match flavor {
            KafkaFlavor::ApacheNative => (
                Self::APACHE_DEFAULT_PORT,
                Self::APACHE_DEFAULT_IMAGE_NAME,
                Self::APACHE_DEFAULT_TAG,
            ),
            KafkaFlavor::Confluent => (
                Self::CONFLUENT_DEFAULT_PORT,
                Self::CONFLUENT_DEFAULT_IMAGE_NAME,
                Self::CONFLUENT_DEFAULT_TAG,
            ),
        };

        let kafka_impl = kafka_impl.unwrap_or_else(|| match flavor {
            KafkaFlavor::ApacheNative => {
                Box::new(KafkaContainerImpl::new(network)) as Box<dyn KafkaImpl>
            }
            KafkaFlavor::Confluent => {
                Box::new(ConfluentKafkaContainerImpl::new(network)) as Box<dyn KafkaImpl>
            }
        });

        let port = port.unwrap_or(default_port);
        let image_name = image_name.unwrap_or_else(|| default_image_name.to_string());
        let image_tag = image_tag.unwrap_or_else(|| default_tag.to_string());

        let mut dep = KafkaDependency::new(
            arena_container::identifier::build("arena-kafka", &identifier),
            kafka_impl,
            port,
            dependencies,
            image_name,
            image_tag,
            container_name,
            topics,
        );

        if let Some(check) = readiness_check {
            dep.set_readiness_check(check);
        }

        dep
    }
}
