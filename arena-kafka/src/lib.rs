pub(crate) const MODULE: &str = "arena-kafka";

pub mod builder;
pub mod kafka_dependency;

pub use crate::builder::KafkaFlavor;
pub use crate::kafka_dependency::topic_creator::TopicCreator;
pub use crate::kafka_dependency::KafkaDependency;
pub use crate::kafka_dependency::KafkaImpl;
pub use crate::kafka_dependency::KAFKA_INTERNAL_DOCKER_PORT;
