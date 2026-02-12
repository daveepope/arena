use async_trait::async_trait;
use arena::component::RunnableComponent;
use crate::builder::ContainerComponentBuilder;

pub struct ContainerComponent {
    pub(crate) endpoint: String,
    pub(crate) children: Option<Vec<Box<dyn RunnableComponent>>>,
    pub(crate) stopped: bool,
}

impl ContainerComponent {
    pub fn new(endpoint: String) -> Self {
        ContainerComponent {
            endpoint,
            children: None,
            stopped: false,
        }
    }

    pub fn builder(endpoint: impl Into<String>) -> ContainerComponentBuilder {
        ContainerComponentBuilder::new(endpoint)
    }
}

#[async_trait]
impl RunnableComponent for ContainerComponent {
    async fn start(&mut self) {
        for child in self.children.iter_mut().flatten() {
            child.start().await;
        }

        log::info!("[Component-{}] starting.", self.endpoint);
        log::info!("[Component-{}] started.", self.endpoint);
    }

    async fn stop(&mut self) {
        if self.stopped {
            return;
        }

        log::info!("[Component-{}] stopping.", self.endpoint);
        log::info!("[Component-{}] stopped.", self.endpoint);

        for child in self.children.iter_mut().flatten().rev() {
            child.stop().await;
        }

        self.stopped = true;
    }

    fn add_child(&mut self, child: Box<dyn RunnableComponent>) {
        self.children.get_or_insert_with(Vec::new).push(child);
    }
}