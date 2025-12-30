use arena::{Arena, Encounter, EncounterTrait, Component, ManagedProcessComponent, Dependency};
use arena_kafka::{KafkaDependency, DockerKafkaImpl};
use arena_postgres::{PostgresDependency, DockerPostgresImpl};

#[tokio::main]
async fn main() {
    let mut postgres_db: Dependency = Box::new(PostgresDependency::new(
        String::from("parent"),
        Box::new(DockerPostgresImpl::new()),
    ));

    let kafka: Dependency = Box::new(KafkaDependency::new(
        String::from("child"),
        Box::new(DockerKafkaImpl::new()),
    ));

    postgres_db.add_child(kafka);

    let dependencies: Vec<Dependency> = vec![postgres_db];

    let components: Vec<Component> = vec![
        Component::Application(ManagedProcessComponent::new("web app".to_string())),
    ];

    let encounter = Encounter::new("End to end happy path", dependencies, components);
    let encounters: Vec<Box<dyn EncounterTrait>> = vec![Box::new(encounter)];

    let mut arena = Arena::new(String::from("Example Arena"), encounters);
    arena.commence().await;
}