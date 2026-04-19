pub mod localstack_dependency;
pub mod builder;

pub use crate::builder::{
    EventBusSpec,
    EventRuleSpec,
    EventRuleTarget,
    EventTargetKind,
    LambdaSpec,
    LocalstackDependencyBuilder,
    QueueSpec,
};
pub use crate::localstack_dependency::LOCALSTACK_INTERNAL_DOCKER_PORT;
pub use crate::localstack_dependency::LocalstackDependency;
pub use crate::localstack_dependency::LocalstackImpl;
pub use crate::localstack_dependency::resource_creator::ResourceCreator;
