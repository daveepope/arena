use std::ops::Drop;
use super::component::Component;
use super::dependency::Dependency;

pub trait EncounterTrait: Send + Sync {
    fn start(&mut self);
    fn stop(&mut self);
}

pub struct Encounter {
    pub name: String,
    dependencies: Vec<Dependency>,
    components: Vec<Component>,
    started: bool
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

impl EncounterTrait for Encounter {
    fn start(&mut self) {
        if self.started { return; }
        println!("[Encounters-{}] starting.", self.name);
        for dep in self.dependencies.iter_mut() {
            dep.start();
        }
        for comp in self.components.iter() {
            comp.start();
        }
        println!("[Encounters-{}] started.", self.name);
        self.started = true;
    }

    fn stop(&mut self) {
        if !self.started { return; }
        println!("[Encounters-{}] stopping.", self.name);
        for comp in self.components.iter_mut().rev() {
            comp.stop();
        }
        for dep in self.dependencies.iter_mut().rev() {
            dep.stop();
        }
        println!("[Encounters-{}] stopped.", self.name);
        self.started = false;
    }
}

impl Drop for Encounter {
    fn drop(&mut self) {
        self.stop();
    }
}