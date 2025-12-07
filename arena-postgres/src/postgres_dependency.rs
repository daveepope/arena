use arena::dependency::RunnableDependency;

pub struct PostgresDependency {
    pub name: String,
    dependencies: Vec<Box<dyn RunnableDependency>>,
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
        if !self.running { return; }
        println!("[Postgres-{}] stopping.", self.name);
        for dep in self.dependencies.iter_mut().rev() {
            dep.stop();
        }
        println!("[Postgres-{}] stopped.", self.name);
        self.running = false;
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.push(dep);
    }
}