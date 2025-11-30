use crate::dependency::{Dependency, RunnableDependency};

pub struct KafkaDependency {
    pub name: String,
    pub dependencies: Vec<Dependency>,
    running: bool
}

impl KafkaDependency {
    pub fn new(name: String) -> Self {
        KafkaDependency { name, dependencies: vec![], running: false }
    }
}

impl RunnableDependency for KafkaDependency {
    fn start(&mut self) {
        if self.running { return; }
        println!("[Kafka-{}] starting.", self.name);
        for dep in self.dependencies.iter_mut() {
            dep.start();
        }
        self.running = true;
        println!("[Kafka-{}] started.", self.name);
    }

    fn stop(&mut self) {
        if(!self.running) { return; }
        println!("[Kafka-{}] stopping.", self.name);
        for dep in self.dependencies.iter_mut().rev() {
            dep.stop();
        }
        println!("[Kafka-{}] stopped.", self.name);
        self.running = false;
    }

    fn add_child_internal(&mut self, dep: Dependency) {
        self.dependencies.push(dep);
    }
}

impl Drop for KafkaDependency {
    fn drop(&mut self) {
        self.stop();
    }
}