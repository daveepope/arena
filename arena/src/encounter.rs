use super::component::Component;
use super::dependency::Dependency;
use super::dependency::RunnableDependency;
use async_trait::async_trait;
use futures::future::join_all;
use std::time::Instant;

#[async_trait]
pub trait EncounterTrait: Send + Sync {
    async fn start(&mut self);
    async fn stop(&mut self);

    fn dependency(&self, _identifier: &str) -> Option<&(dyn RunnableDependency + '_)> {
        None
    }

    fn dependency_mut(&mut self, _identifier: &str) -> Option<&mut (dyn RunnableDependency + '_)> {
        None
    }
}

#[async_trait]
pub trait SetupHandler: Send + Sync {
    async fn setup(&self, dependencies: &[Dependency]);
}

pub type Setup = Box<dyn SetupHandler>;

pub struct Encounter {
    pub name: String,
    dependencies: Vec<Dependency>,
    components: Vec<Component>,
    setup_handlers: Vec<Setup>,
    started: bool,
}

impl Encounter {
    pub fn new(name: &str, dependencies: Vec<Dependency>, components: Vec<Component>) -> Self {
        Encounter {
            name: name.to_string(),
            dependencies,
            components,
            setup_handlers: Vec::new(),
            started: false,
        }
    }

    pub fn with_dependency_setup_handler(mut self, handler: Setup) -> Self {
        self.setup_handlers.push(handler);
        self
    }
}

#[async_trait]
impl EncounterTrait for Encounter {
    async fn start(&mut self) {
        if self.started {
            return;
        }

        log::info!("[Encounters-{}] starting.", self.name);
        let sw = Instant::now();

        let deps = std::mem::take(&mut self.dependencies);

        let mut started = join_all(deps.into_iter().enumerate().map(|(i, mut dep)| async move {
            dep.start().await;
            (i, dep)
        }))
        .await;

        started.sort_by_key(|(i, _)| *i);
        self.dependencies = started.into_iter().map(|(_, dep)| dep).collect();

        for handler in &self.setup_handlers {
            handler.setup(&self.dependencies).await;
        }

        let comps = std::mem::take(&mut self.components);

        let mut started_comps = join_all(comps.into_iter().enumerate().map(|(i, mut comp)| async move {
            comp.start().await;
            (i, comp)
        }))
        .await;

        started_comps.sort_by_key(|(i, _)| *i);
        self.components = started_comps.into_iter().map(|(_, comp)| comp).collect();

        log::debug!(
            "[Encounters-{}] start complete in {:?}.",
            self.name,
            sw.elapsed()
        );
        log::info!("[Encounters-{}] started.", self.name);
        self.started = true;
    }

    async fn stop(&mut self) {
        if !self.started {
            return;
        }

        log::info!("[Encounters-{}] stopping.", self.name);
        let sw = Instant::now();

        let comps = std::mem::take(&mut self.components);

        let mut stopped_comps = join_all(comps.into_iter().enumerate().map(|(i, mut comp)| async move {
            comp.stop().await;
            (i, comp)
        }))
        .await;

        stopped_comps.sort_by_key(|(i, _)| *i);
        self.components = stopped_comps.into_iter().map(|(_, comp)| comp).collect();

        let deps = std::mem::take(&mut self.dependencies);

        let mut stopped = join_all(deps.into_iter().enumerate().map(|(i, mut dep)| async move {
            dep.stop().await;
            (i, dep)
        }))
        .await;

        stopped.sort_by_key(|(i, _)| *i);
        self.dependencies = stopped.into_iter().map(|(_, dep)| dep).collect();

        log::debug!(
            "[Encounters-{}] stop complete in {:?}.",
            self.name,
            sw.elapsed()
        );
        log::info!("[Encounters-{}] stopped.", self.name);
        self.started = false;
    }

    fn dependency(&self, identifier: &str) -> Option<&(dyn RunnableDependency + '_)> {
        self.dependencies
            .iter()
            .map(|d| d.as_ref())
            .find(|d| d.identifier() == identifier)
    }

    fn dependency_mut(&mut self, identifier: &str) -> Option<&mut (dyn RunnableDependency + '_)> {
        for dep in &mut self.dependencies {
            if dep.identifier() == identifier {
                return Some(dep.as_mut());
            }
        }
        None
    }
}
