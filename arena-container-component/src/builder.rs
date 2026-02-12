use arena::Component;
use crate::executable_component::ExecutableComponent;

pub struct DockerComponentBuilder {
    endpoint: String,
    children: Option<Vec<Component>>,
}

impl DockerComponentBuilder {
    pub(crate) fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            children: None,
        }
    }

    pub fn with_child_components(mut self, children: Vec<Component>) -> Self {
        self.children = Some(children);
        self
    }

    pub fn build(self) -> ExecutableComponent {
        ExecutableComponent {
            endpoint: self.endpoint,
            children: self.children,
            stopped: false,
        }
    }
}