use crate::dependency::{Dependency, RunnableDependency};

pub struct PostgresDependency {
    pub name: String,
    pub dependencies: Vec<Dependency>,
    running: bool
}

impl PostgresDependency {
    pub fn new(name: String) -> Self {
        PostgresDependency { name, dependencies: vec![], running: false }
    }
}

impl RunnableDependency for PostgresDependency {
    fn start(&mut self) {
        if self.running { return; }
        println!("[Postgres-{}] starting.", self.name);
        for dep in self.dependencies.iter_mut() {
            dep.start();
        }
        self.running = true;
        println!("[Postgres-{}] started.", self.name);
    }

    fn stop(&mut self) {
        if(!self.running) { return; }
        println!("[Postgres-{}] stopping.", self.name);
        for dep in self.dependencies.iter_mut().rev() {
            dep.stop();
        }
        println!("[Postgres-{}] stopped.", self.name);
        self.running = false;
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