use arena::{Arena, Encounter, EncounterTrait, Component, ManagedProcessComponent, Dependency};
use arena_kafka::kafka_dependency::InternalKafkaTestContainerImpl;
use arena_kafka::KafkaDependency;
use arena_postgres::postgres_dependency::InternalPostgresTestContainerImpl;
use arena_postgres::PostgresDependency;
use env_logger::Env;


#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let postgres_db: Dependency = Box::new(PostgresDependency::new(
        String::from("parent"),
        Box::new(InternalPostgresTestContainerImpl::new()),
    ));

    let kafka: Dependency = Box::new(KafkaDependency::new(
        String::from("child"),
        Box::new(InternalKafkaTestContainerImpl::new()),
    ));

    //postgres_db.add_child(kafka);

    let dependencies: Vec<Dependency> = vec![postgres_db, kafka];

    let components: Vec<Component> = vec![
        Component::Application(ManagedProcessComponent::new("web app".to_string())),
    ];

    let encounter = Encounter::new("End to end happy path", dependencies, components);
    let encounters: Vec<Box<dyn EncounterTrait>> = vec![Box::new(encounter)];

    let mut arena = Arena::new(String::from("Example Arena"), encounters);
    arena.commence().await;
}