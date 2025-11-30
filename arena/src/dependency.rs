use crate::kafka_dependency::KafkaDependency;
use crate::postgres_dependency::PostgresDependency;

pub enum Dependency {
    PostgresDependency(PostgresDependency),
    KafkaDependency(KafkaDependency)
}

pub trait RunnableDependency {
    fn start(&mut self);
    fn stop(&mut self);
    fn add_child_internal(&mut self, dep: Dependency);
}

impl Dependency {
    pub fn start(&mut self) {
        match self {
            Dependency::PostgresDependency(dep) => dep.start(),
            Dependency::KafkaDependency(dep) => dep.start()
        }
    }
    pub fn stop(&mut self) {
        match self {
            Dependency::PostgresDependency(dep) => dep.stop(),
            Dependency::KafkaDependency(dep) => dep.stop()
        }
    }

    pub fn add_child(&mut self, dep: Dependency) {
        match self {
            Dependency::PostgresDependency(db_dep) => db_dep.add_child_internal(dep),
            Dependency::KafkaDependency(db_dep) => db_dep.add_child_internal(dep)
        }
    }
}


