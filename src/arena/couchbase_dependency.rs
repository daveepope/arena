use crate::arena::dependency::RunnableDependency;
use crate::Dependency;

pub struct CouchbaseDependency {
    pub name: String,
    pub dependencies: Vec<Dependency>,
    started: bool
}

impl CouchbaseDependency {
    pub fn new(name: String) -> Self {
        CouchbaseDependency { name, dependencies: vec![], started: false }
    }
}

impl RunnableDependency for CouchbaseDependency {
    fn start(&mut self) {
        if self.started { return; }
        println!("[{}] (Couchbase) Starting connection.", self.name);
        for dep in self.dependencies.iter_mut() {
            dep.start();
        }
        self.started = true;
        println!("[{}] (Couchbase) Connection started.", self.name);
    }

    fn stop(&mut self) {
        println!("[{}] (Couchbase) Stopping connection.", self.name);
        for dep in self.dependencies.iter_mut().rev() {
            dep.stop();
        }
        println!("[{}] (Couchbase) Connection stopped.", self.name);
    }

    fn add_child_internal(&mut self, dep: Dependency) {
        self.dependencies.push(dep);
    }
}

impl Drop for CouchbaseDependency {
    fn drop(&mut self) {
        self.stop();
    }
}