use super::component::Component;
use super::dependency::Dependency;
use async_trait::async_trait;
use futures::future::join_all;

#[async_trait]
pub trait EncounterTrait: Send + Sync {
    async fn start(&mut self);
    async fn stop(&mut self);
}

pub struct Encounter {
    pub name: String,
    dependencies: Vec<Dependency>,
    components: Vec<Component>,
    started: bool,
}

impl Encounter {
    pub fn new(name: &str, dependencies: Vec<Dependency>, components: Vec<Component>) -> Self {
        Encounter {
            name: name.to_string(),
            dependencies,
            components,
            started: false,
        }
    }
}

#[async_trait]
impl EncounterTrait for Encounter {
    async fn start(&mut self) {
        if self.started {
            return;
        }

        log::info!("[Encounters-{}] starting.", self.name);

        let deps = std::mem::take(&mut self.dependencies);

        let mut started = join_all(deps.into_iter().enumerate().map(|(i, mut dep)| async move {
            dep.start().await;
            (i, dep)
        }))
        .await;

        started.sort_by_key(|(i, _)| *i);
        self.dependencies = started.into_iter().map(|(_, dep)| dep).collect();

        for comp in self.components.iter() {
            comp.start();
        }

        log::info!("[Encounters-{}] started.", self.name);
        self.started = true;
    }

    async fn stop(&mut self) {
        if !self.started {
            return;
        }

        log::info!("[Encounters-{}] stopping.", self.name);

        for comp in self.components.iter_mut().rev() {
            comp.stop();
        }

        for dep in self.dependencies.iter_mut().rev() {
            dep.stop().await;
        }

        log::info!("[Encounters-{}] stopped.", self.name);
        self.started = false;
    }
}
