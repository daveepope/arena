use super::component::Component;
use super::dependency::Dependency;
use async_trait::async_trait;

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
    pub fn new(
        name: &str,
        dependencies: Vec<Dependency>,
        components: Vec<Component>,
    ) -> Self {
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

        println!("[Encounters-{}] starting.", self.name);

        for dep in self.dependencies.iter_mut() {
            dep.start().await;
        }

        for comp in self.components.iter() {
            comp.start();
        }

        println!("[Encounters-{}] started.", self.name);
        self.started = true;
    }

    async fn stop(&mut self) {
        if !self.started {
            return;
        }

        println!("[Encounters-{}] stopping.", self.name);

        for comp in self.components.iter_mut().rev() {
            comp.stop();
        }

        for dep in self.dependencies.iter_mut().rev() {
            dep.stop().await;
        }

        println!("[Encounters-{}] stopped.", self.name);
        self.started = false;
    }
}