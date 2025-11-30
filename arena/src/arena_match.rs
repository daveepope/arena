use std::ops::Drop;
use super::component::Component;
use super::dependency::Dependency;

pub struct ArenaMatch {
    pub name: String,
    dependencies: Vec<Dependency>,
    components: Vec<Component>,
    started: bool
}

impl ArenaMatch {
    pub fn new(
        name: &str,
        dependencies: Vec<Dependency>,
        components: Vec<Component>,
    ) -> Self {
        ArenaMatch {
            name: name.to_string(),
            dependencies,
            components,
            started: false,
        }
    }

    pub fn start(&mut self) {
        if self.started { return; }
        println!("[Match:{}] Starting.", self.name);
        for dep in self.dependencies.iter_mut() {
            dep.start();
        }
        for comp in self.components.iter() {
            comp.start();
        }
        println!("[Match:{}] Started.", self.name);
        self.started = true;
    }

    pub fn stop(&mut self) {
        if !self.started { return; }
        println!("[Match:{}] Stopping.", self.name);
        for comp in self.components.iter_mut().rev() {
            comp.stop();
        }
        for dep in self.dependencies.iter_mut().rev() {
            dep.stop();
        }
        println!("[Match:{}] Stopped.", self.name);
        self.started = false;
    }
}

impl Drop for ArenaMatch {
    fn drop(&mut self) {
        self.stop();
    }
}
