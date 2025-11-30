use arena::{Arena, ArenaMatch, Component, ManagedProcessComponent, Dependency, PostgresDependency, KafkaDependency};

fn main() {
    let mut postgres_db = Dependency::PostgresDependency(PostgresDependency::new(
        String::from("postgres_db")
    ));

    let kafka = Dependency::KafkaDependency(KafkaDependency::new(
        String::from("kafka")
    ));

    postgres_db.add_child(kafka);

    let dependencies: Vec<Dependency> = vec![postgres_db];
    let component: Vec<Component> = vec![
        Component::Application(ManagedProcessComponent::new("web app".to_string())),
    ];

    let mut arena = Arena::new(String::from(""), vec![ArenaMatch::new("End too end happy path", dependencies, component)]);
    arena.commence();
}