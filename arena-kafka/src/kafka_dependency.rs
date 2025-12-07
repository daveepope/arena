use arena::dependency::RunnableDependency;

pub struct KafkaDependency {
    pub name: String,
    dependencies: Vec<Box<dyn RunnableDependency>>,
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
        if !self.running { return; }
        println!("[Kafka-{}] stopping.", self.name);
        for dep in self.dependencies.iter_mut().rev() {
            dep.stop();
        }
        println!("[Kafka-{}] stopped.", self.name);
        self.running = false;
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.push(dep);
    }
}