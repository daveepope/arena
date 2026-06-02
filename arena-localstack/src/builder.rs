use std::path::PathBuf;

use crate::localstack_dependency::container_impl::LocalstackContainerImpl;
use crate::localstack_dependency::{LocalstackDependency, LocalstackImpl};
use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;

#[derive(Debug, Clone)]
pub struct QueueSpec {
    pub name: String,
    pub fifo: bool,
}

#[derive(Debug, Clone)]
pub struct LambdaSpec {
    pub name: String,
    pub runtime: String,
    pub handler: String,
    pub source_dir: PathBuf,
    pub environment: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct EventBusSpec {
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum EventTargetKind {
    SqsQueue { queue_name: String },
    Lambda { function_name: String },
}

#[derive(Debug, Clone)]
pub struct EventRuleTarget {
    pub target_id: String,
    pub kind: EventTargetKind,
}

#[derive(Debug, Clone)]
pub struct EventRuleSpec {
    pub name: String,
    pub event_bus: Option<String>,
    pub event_pattern: String,
    pub targets: Vec<EventRuleTarget>,
}

pub struct LocalstackDependencyBuilder {
    identifier: String,
    localstack_impl: Option<Box<dyn LocalstackImpl>>,
    port: Option<u16>,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    image_name: Option<String>,
    image_tag: Option<String>,
    container_name: Option<String>,
    network: Option<String>,
    readiness_check: Option<Box<dyn ReadinessCheck>>,
    services: Vec<String>,
    queues: Vec<QueueSpec>,
    lambdas: Vec<LambdaSpec>,
    event_buses: Vec<EventBusSpec>,
    event_rules: Vec<EventRuleSpec>,
}

impl LocalstackDependencyBuilder {
    const AUTO_HOST_PORT: u16 = 0;
    const DEFAULT_IMAGE_NAME: &'static str = "localstack/localstack";
    const DEFAULT_IMAGE_TAG: &'static str = "4.5";

    pub(crate) fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            localstack_impl: None,
            port: None,
            dependencies: None,
            image_name: None,
            image_tag: None,
            container_name: None,
            network: None,
            readiness_check: None,
            services: Vec::new(),
            queues: Vec::new(),
            lambdas: Vec::new(),
            event_buses: Vec::new(),
            event_rules: Vec::new(),
        }
    }

    pub fn with_impl<W>(mut self, wrapper: W) -> Self
    where
        W: LocalstackImpl + 'static,
    {
        self.localstack_impl = Some(Box::new(wrapper));
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn with_child_dependencies(
        mut self,
        dependencies: Vec<Box<dyn RunnableDependency>>,
    ) -> Self {
        self.dependencies = Some(dependencies);
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

    pub fn with_readiness_check<W>(mut self, check: W) -> Self
    where
        W: ReadinessCheck + 'static,
    {
        self.readiness_check = Some(Box::new(check));
        self
    }

    pub fn with_service(mut self, service: impl Into<String>) -> Self {
        self.services.push(service.into());
        self
    }

    pub fn with_services<I, S>(mut self, services: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.services.extend(services.into_iter().map(Into::into));
        self
    }

    pub fn with_queue(mut self, name: impl Into<String>) -> Self {
        self.queues.push(QueueSpec {
            name: name.into(),
            fifo: false,
        });
        self
    }

    pub fn with_fifo_queue(mut self, name: impl Into<String>) -> Self {
        self.queues.push(QueueSpec {
            name: name.into(),
            fifo: true,
        });
        self
    }

    pub fn with_queue_spec(mut self, spec: QueueSpec) -> Self {
        self.queues.push(spec);
        self
    }

    pub fn with_lambda(mut self, spec: LambdaSpec) -> Self {
        self.lambdas.push(spec);
        self
    }

    pub fn with_event_bus(mut self, name: impl Into<String>) -> Self {
        self.event_buses.push(EventBusSpec { name: name.into() });
        self
    }

    pub fn with_event_rule(mut self, rule: EventRuleSpec) -> Self {
        self.event_rules.push(rule);
        self
    }

    pub fn build(self) -> LocalstackDependency {
        let LocalstackDependencyBuilder {
            identifier,
            localstack_impl,
            port,
            dependencies,
            image_name,
            image_tag,
            container_name,
            network,
            readiness_check,
            services,
            queues,
            lambdas,
            event_buses,
            event_rules,
        } = self;

        let localstack_impl = localstack_impl.unwrap_or_else(|| {
            Box::new(LocalstackContainerImpl::new(network)) as Box<dyn LocalstackImpl>
        });

        let port = port.unwrap_or(Self::AUTO_HOST_PORT);
        let image_name = image_name.unwrap_or_else(|| Self::DEFAULT_IMAGE_NAME.to_string());
        let image_tag = image_tag.unwrap_or_else(|| Self::DEFAULT_IMAGE_TAG.to_string());

        let mut dep = LocalstackDependency::new(
            arena_container::identifier::build("arena-localstack", &identifier),
            localstack_impl,
            port,
            dependencies,
            image_name,
            image_tag,
            container_name,
            services,
            queues,
            lambdas,
            event_buses,
            event_rules,
        );

        if let Some(check) = readiness_check {
            dep.set_readiness_check(check);
        }

        dep
    }
}
