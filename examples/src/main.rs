use arena::{Arena, Encounter, Component, ManagedProcessComponent, Dependency};

use arena_kafka::KafkaDependency;
use arena_postgres::PostgresDependency;

fn main() {
    let mut postgres_db: Dependency = Box::new(PostgresDependency::new(
        String::from("parent")
    ));

    let kafka: Dependency = Box::new(KafkaDependency::new(
        String::from("child")
    ));

    postgres_db.add_child(kafka);

    let dependencies: Vec<Dependency> = vec![postgres_db];
    let component: Vec<Component> = vec![
        Component::Application(ManagedProcessComponent::new("web app".to_string())),
    ];

    let encounter: Encounter = Encounter::new("End to end happy path suite", dependencies, component);

    let mut arena = Arena::new(
        String::from("Example-Arena"),
        vec![encounter]
    );
    arena.commence();
}