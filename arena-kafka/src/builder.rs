use arena::dependency::RunnableDependency;
use crate::kafka_dependency::container_impl::{ConfluentKafkaContainerImpl, KafkaContainerImpl };
use crate::kafka_dependency::{KafkaDependency, KafkaImpl};

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
    image_tag: Option<String>,
    container_name: Option<String>,
}

impl KafkaDependencyBuilder {
    const APACHE_DEFAULT_PORT: u16 = 9092;
    const APACHE_DEFAULT_TAG: &'static str = "3.8.0";

    const CONFLUENT_DEFAULT_PORT: u16 = 9093;
    const CONFLUENT_DEFAULT_TAG: &'static str = "6.1.1";

    pub(crate) fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            kafka_impl: None,
            flavor: KafkaFlavor::ApacheNative,
            port: None,
            dependencies: None,
            image_tag: None,
            container_name: None,
        }
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

    pub fn with_image_tag(mut self, image_tag: impl Into<String>) -> Self {
        self.image_tag = Some(image_tag.into());
        self
    }

    pub fn with_image(self, image_tag: impl Into<String>) -> Self {
        self.with_image_tag(image_tag)
    }

    pub fn with_container_name(mut self, container_name: impl Into<String>) -> Self {
        self.container_name = Some(container_name.into());
        self
    }

    pub fn with_container_tag(self, image_tag: impl Into<String>) -> Self {
        self.with_image_tag(image_tag)
    }

    pub fn build(self) -> KafkaDependency {
        let KafkaDependencyBuilder {
            identifier,
            kafka_impl,
            flavor,
            port,
            dependencies,
            image_tag,
            container_name,
        } = self;

        let (default_port, default_tag) = match flavor {
            KafkaFlavor::ApacheNative => (Self::APACHE_DEFAULT_PORT, Self::APACHE_DEFAULT_TAG),
            KafkaFlavor::Confluent => (Self::CONFLUENT_DEFAULT_PORT, Self::CONFLUENT_DEFAULT_TAG),
        };

        let kafka_impl = kafka_impl.unwrap_or_else(|| match flavor {
            KafkaFlavor::ApacheNative => Box::new(KafkaContainerImpl::new()) as Box<dyn KafkaImpl>,
            KafkaFlavor::Confluent => {
                Box::new(ConfluentKafkaContainerImpl::new()) as Box<dyn KafkaImpl>
            }
        });

        let port = port.unwrap_or(default_port);
        let image_tag = image_tag.unwrap_or_else(|| default_tag.to_string());

        KafkaDependency::new(identifier, kafka_impl, port, dependencies, image_tag, container_name)
    }
}

