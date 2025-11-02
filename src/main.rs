pub struct DatabaseDependency {
    name: String,
}
impl DatabaseDependency {
    pub fn start(&self) {
        println!("[{}] Starting Database connection...", self.name);
    }
    pub fn stop(&self) {
        println!("[{}] Stopping Database connection...", self.name);
    }
}

pub struct ApplicationComponent {
    endpoint: String,
}

impl ApplicationComponent {
    pub fn start(&self) {
        println!("[Application] Connecting to endpoint: {}", self.endpoint);
    }
    pub fn stop(&self) {
        println!("[Application] Disconnecting from endpoint: {}", self.endpoint);
    }
}

pub enum Dependency {
    Database(DatabaseDependency),
}

impl Dependency {
    pub fn start(&self) {
        match self {
            Dependency::Database(dep) => dep.start(),
        }
    }
    pub fn stop(&self) {
        match self {
            Dependency::Database(dep) => dep.stop(),
        }
    }
}

pub enum Component {
    Application(ApplicationComponent),
}

impl Component {
    pub fn start(&self) {
        match self {
            Component::Application(comp) => comp.start(),
        }
    }
    pub fn stop(&self) {
        match self {
            Component::Application(comp) => comp.stop(),
        }
    }
}

pub struct Match {
    pub name: String,
    dependencies: Vec<Dependency>,
    components: Vec<Component>,
}

impl Match {
    pub fn new(
        name: &str,
        dependencies: Vec<Dependency>,
        components: Vec<Component>,
    ) -> Self {
        Match {
            name: name.to_string(),
            dependencies,
            components,
        }
    }

    pub fn start(&self) {
        println!("--- Match '{}' is STARTING ---", self.name);
        for dep in self.dependencies.iter() {
            dep.start();
        }
        for comp in self.components.iter() {
            comp.start();
        }
        println!("--- Match '{}' Started ---\n", self.name);
    }

    pub fn stop(&self) {
        println!("--- Match '{}' is STOPPING ---", self.name);
        for comp in self.components.iter().rev() {
            comp.stop();
        }
        for dep in self.dependencies.iter().rev() {
            dep.stop();
        }
        println!("--- Match '{}' Stopped ---\n", self.name);
    }
}

impl Drop for Match {
    fn drop(&mut self) {
        println!("Dropping {}", self.name);
    }
}

pub struct Arena {
    name: String,
    pub matches: Vec<Match>
}

impl Arena {
    pub fn new(name: String, matches: Vec<Match>) -> Self {
        Arena {
            name,
            matches
        }
    }

    pub fn start(&self) {
        println!("\n======= ARENA [{}] STARTING ALL MATCHES =======", self.name);
        for m in self.matches.iter() {
            m.start();
        }
        println!("======= ARENA [{}] READY =======\n", self.name);
    }

    pub fn stop(&self) {
        println!("\n======= ARENA [{}] STOPPING ALL MATCHES =======", self.name);
        for m in self.matches.iter().rev() {
            m.stop();
        }
        println!("======= ARENA [{}] SHUTDOWN COMPLETE =======\n", self.name);
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        println!("Dropping {}", self.name);
    }
}

fn main() {
    let deps1: Vec<Dependency> = vec![
        Dependency::Database(DatabaseDependency { name: "DB_Prod".to_string() }),
    ];
    let comps1: Vec<Component> = vec![
        Component::Application(ApplicationComponent { endpoint: "web app".to_string() }),
    ];
    let match1 = Match::new("end too end happy path", deps1, comps1);

    let deps2: Vec<Dependency> = vec![
        Dependency::Database(DatabaseDependency { name: "DB_Prod".to_string() }),
    ];
    let comps2: Vec<Component> = vec![
        Component::Application(ApplicationComponent { endpoint: "web app".to_string() }),
    ];
    let match2 = Match::new("end too end error path", deps2, comps2);

    let matches: Vec<Match> = vec![match1, match2];
    let arena = Arena::new(String::from("Component Test Suite"), matches);
    arena.start();
    arena.stop();
}