use arena::{ClosedArena, Encounter, EncounterTrait, Component, ManagedProcessComponent, Dependency};
use arena_kafka::kafka_dependency::InternalKafkaTestContainerImpl;
use arena_kafka::KafkaDependency;
use arena_postgres::PostgresDependency;
use env_logger::Env;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let startup_sql_scripts = vec![
        include_str!("../resources/instrument_reading_db_schema.sql").to_string(),
    ];

    let postgres_db: Dependency = Box::new(
        PostgresDependency::builder("arena example database")
            .with_container_name("postgres:14.20-trixie")
            .with_port(4444)
            .with_database_name("my_database")
            .with_database_username("my_user")
            .with_database_password("my_password")
            .with_startup_sql_scripts(startup_sql_scripts)
            .build());

    let kafka: Dependency = Box::new(KafkaDependency::new(
        String::from("kafka custom dependency name"),
        Box::new(InternalKafkaTestContainerImpl::new()),
    ));

    let dependencies: Vec<Dependency> = vec![postgres_db, kafka];

    let components: Vec<Component> = vec![
        Component::Application(ManagedProcessComponent::new("web app".to_string())),
    ];

    let encounter = Encounter::new("End to end happy path", dependencies, components);
    let encounters: Vec<Box<dyn EncounterTrait>> = vec![Box::new(encounter)];

    let closed = ClosedArena::new(String::from("Example Arena"), encounters);

    let open = closed.open().await;
    let _closed = open.close().await;
}