pub(crate)
use crate::arena::postgres_dependency::PostgresDependency;
use crate::arena::couchbase_dependency::CouchbaseDependency;

pub enum Dependency {
    PostgresDependency(PostgresDependency),
    CouchbaseDependency(CouchbaseDependency)
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
            Dependency::CouchbaseDependency(dep) => dep.start()
        }
    }
    pub fn stop(&mut self) {
        match self {
            Dependency::PostgresDependency(dep) => dep.stop(),
            Dependency::CouchbaseDependency(dep) => dep.stop()
        }
    }

    pub fn add_child(&mut self, dep: Dependency) {
        match self {
            Dependency::PostgresDependency(db_dep) => db_dep.add_child_internal(dep),
            Dependency::CouchbaseDependency(db_dep) => db_dep.add_child_internal(dep)
        }
    }
}


