use arena::{Arena, Encounter, EncounterTrait, Component, ManagedProcessComponent, Dependency};
use arena_kafka::KafkaDependency;
use arena_postgres::PostgresDependency;

fn main() {
    // Create concrete dependencies
    let mut postgres_db: Dependency = Box::new(PostgresDependency::new(
        String::from("postgres_db")
    ));

    let kafka: Dependency = Box::new(KafkaDependency::new(
        String::from("kafka")
    ));

    // Add kafka as a child of postgres
    postgres_db.add_child(kafka);

    let dependencies: Vec<Dependency> = vec![postgres_db];

    let components: Vec<Component> = vec![
        Component::Application(ManagedProcessComponent::new("web app".to_string())),
    ];

    // Create encounter with concrete dependencies
    let encounter = Encounter::new("End to end happy path", dependencies, components);
    let encounters: Vec<Box<dyn EncounterTrait>> = vec![Box::new(encounter)];

    let mut arena = Arena::new(String::from(""), encounters);
    arena.commence();
}