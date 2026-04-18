use super::component::Component;
use super::dependency::Dependency;
use super::dependency::RunnableDependency;
use async_trait::async_trait;
use futures::future::join_all;
use std::time::Instant;

#[async_trait]
pub trait MatchTrait: Send + Sync {
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
pub trait OnDependencyStartup: Send + Sync {
    async fn on_dependency_startup(&self, dependencies: &[Dependency]);
}

pub struct Match {
    pub name: String,
    dependencies: Vec<Dependency>,
    components: Vec<Component>,
    on_dependency_startup_handlers: Vec<Box<dyn OnDependencyStartup>>,
    started: bool,
}

impl Match {
    pub fn new(name: &str, dependencies: Vec<Dependency>, components: Vec<Component>) -> Self {
        Match {
            name: name.to_string(),
            dependencies,
            components,
            on_dependency_startup_handlers: Vec::new(),
            started: false,
        }
    }

    pub fn with_on_dependency_startup_handler(
        mut self,
        handler: Box<dyn OnDependencyStartup>,
    ) -> Self {
        self.on_dependency_startup_handlers.push(handler);
        self
    }
}

#[async_trait]
impl MatchTrait for Match {
    async fn start(&mut self) {
        if self.started {
            return;
        }

        log::info!("[Match-{}] starting.", self.name);
        let sw = Instant::now();

        let deps = std::mem::take(&mut self.dependencies);

        let mut started = join_all(deps.into_iter().enumerate().map(|(i, mut dep)| async move {
            dep.start().await;
            (i, dep)
        }))
        .await;

        started.sort_by_key(|(i, _)| *i);
        self.dependencies = started.into_iter().map(|(_, dep)| dep).collect();

        for handler in &self.on_dependency_startup_handlers {
            handler.on_dependency_startup(&self.dependencies).await;
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
            "[Match-{}] start complete in {:?}.",
            self.name,
            sw.elapsed()
        );
        log::info!("[Match-{}] started.", self.name);
        self.started = true;
    }

    async fn stop(&mut self) {
        if !self.started {
            return;
        }

        log::info!("[Match-{}] stopping.", self.name);
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
            "[Match-{}] stop complete in {:?}.",
            self.name,
            sw.elapsed()
        );
        log::info!("[Match-{}] stopped.", self.name);
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
