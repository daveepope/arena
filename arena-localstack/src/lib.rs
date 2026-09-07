pub(crate) const MODULE: &str = "arena-localstack";

pub mod builder;
pub mod localstack_dependency;
pub mod managed_playbook;
pub mod playbook;

pub use crate::builder::{
    EventBusSpec, EventRuleSpec, EventRuleTarget, EventTargetKind, LambdaSpec,
    LocalstackDependencyBuilder, QueueSpec,
};
pub use crate::localstack_dependency::resource_creator::ResourceCreator;
pub use crate::localstack_dependency::LocalstackDependency;
pub use crate::localstack_dependency::LocalstackImpl;
pub use crate::localstack_dependency::LOCALSTACK_INTERNAL_DOCKER_PORT;
pub use crate::managed_playbook::ManagedLocalstackPlaybook;
pub use crate::playbook::{ActivePlaybook, Playbook};
