use crate::arena::dependency::RunnableDependency;
use crate::Dependency;

pub struct PostgresDependency {
    pub name: String,
    pub dependencies: Vec<Dependency>,
    started: bool
}

impl PostgresDependency {
    pub fn new(name: String) -> Self {
        PostgresDependency { name, dependencies: vec![], started: false }
    }
}

impl crate::arena::dependency::RunnableDependency for PostgresDependency {
    fn start(&mut self) {
        if self.started { return; }
        println!("[{}] (DB) Starting connection.", self.name);
        for dep in self.dependencies.iter_mut() {
            dep.start();
        }
        self.started = true;
        println!("[{}] (DB) Connection started.", self.name);
    }

    fn stop(&mut self) {
        println!("[{}] Stopping connection.", self.name);
        for dep in self.dependencies.iter_mut().rev() {
            dep.stop();
        }
        println!("[{}] (DB) Connection stopped.", self.name);
    }

    fn add_child_internal(&mut self, dep: Dependency) {
        self.dependencies.push(dep);
    }
}

impl Drop for PostgresDependency {
    fn drop(&mut self) {
        self.stop();
    }
}