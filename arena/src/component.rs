use async_trait::async_trait;

#[async_trait]
pub trait RunnableComponent: Send + Sync {
    async fn start(&mut self);
    async fn stop(&mut self);
    fn add_child(&mut self, child: Box<dyn RunnableComponent>);
}

pub type Component = Box<dyn RunnableComponent>;

pub struct ExecutableComponentBuilder {
    endpoint: String,
    children: Option<Vec<Component>>,
}

impl ExecutableComponentBuilder {
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

pub struct ExecutableComponent {
    endpoint: String,
    children: Option<Vec<Box<dyn RunnableComponent>>>,
    stopped: bool,
}

impl ExecutableComponent {
    pub fn new(endpoint: String) -> Self {
        ExecutableComponent {
            endpoint,
            children: None,
            stopped: false,
        }
    }

    pub fn builder(endpoint: impl Into<String>) -> ExecutableComponentBuilder {
        ExecutableComponentBuilder::new(endpoint)
    }
}

#[async_trait]
impl RunnableComponent for ExecutableComponent {
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
