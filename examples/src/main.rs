use arena::{Arena, ArenaMatch, Component, ManagedProcessComponent, Dependency, PostgresDependency, CouchbaseDependency};

fn main() {
    let mut postgres_db = Dependency::PostgresDependency(PostgresDependency::new(
        String::from("postgres_db")
    ));

    let couchbase_cache = Dependency::CouchbaseDependency(CouchbaseDependency::new(
        String::from("couchbase_cache")
    ));

    postgres_db.add_child(couchbase_cache);

    let dependencies: Vec<Dependency> = vec![postgres_db];
    let component: Vec<Component> = vec![
        Component::Application(ManagedProcessComponent::new("web app".to_string())),
    ];

    let mut arena = Arena::new(String::from("Component Test Suite"), vec![ArenaMatch::new("end too end happy path", dependencies, component)]);
    arena.commence();
}