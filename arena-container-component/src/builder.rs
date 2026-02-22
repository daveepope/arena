use arena::Component;
use crate::container_component::ContainerComponent;

pub struct ContainerComponentBuilder {
    endpoint: String,
    children: Option<Vec<Component>>,
}

impl ContainerComponentBuilder {
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

    pub fn build(self) -> ContainerComponent {
        ContainerComponent {
            endpoint: self.endpoint,
            children: self.children,
            stopped: false,
        }
    }
}