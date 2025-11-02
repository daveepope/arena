pub struct DatabaseDependency {
    pub name: String,
    pub dependencies: Vec<Dependency>,
    stopped: bool,
}
impl DatabaseDependency {
    pub fn new(name: String, dependencies: Vec<Dependency>) -> Self {
        DatabaseDependency { name, dependencies, stopped: false }
    }

    pub fn add_child_internal(&mut self, dep: Dependency) {
        self.dependencies.push(dep);
    }

    pub fn start(&self) {
        println!("[{}] (DB) Starting connection.", self.name);
        for dep in self.dependencies.iter() {
            dep.start();
        }
        println!("[{}] (DB) Connection started.", self.name);
    }

    pub fn stop(&mut self) {
        if self.stopped { return; }
        println!("[{}] Stopping connection.", self.name);
        for dep in self.dependencies.iter_mut().rev() {
            dep.stop();
        }
        println!("[{}] (DB) Connection stopped.", self.name);
        self.stopped = true;
    }
}

impl Drop for DatabaseDependency {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct ApplicationComponent {
    endpoint: String,
    stopped: bool,
}
impl ApplicationComponent {
    pub fn start(&self) {
        println!("[Application:{}] Starting.", self.endpoint);
        println!("[Application:{}] Started.", self.endpoint);
    }
    pub fn stop(&mut self) {
        if self.stopped { return; }
        println!("[Application:{}] Stopping.", self.endpoint);
        println!("[Application:{}] Stopped.", self.endpoint);
        self.stopped = true;
    }
}

impl Drop for ApplicationComponent {
    fn drop(&mut self) {
        self.stop();
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
    pub fn stop(&mut self) {
        match self {
            Dependency::Database(dep) => dep.stop(),
        }
    }

    pub fn add_child(&mut self, dep: Dependency) {
        match self {
            Dependency::Database(db_dep) => db_dep.add_child_internal(dep),
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
    pub fn stop(&mut self) {
        match self {
            Component::Application(comp) => comp.stop(),
        }
    }
}


pub struct Match {
    pub name: String,
    dependencies: Vec<Dependency>,
    components: Vec<Component>,
    stopped: bool,
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
            stopped: false,
        }
    }

    pub fn start(&self) {
        println!("[Match:{}] Starting.", self.name);
        for dep in self.dependencies.iter() {
            dep.start();
        }
        for comp in self.components.iter() {
            comp.start();
        }
        println!("[Match:{}] Started.", self.name);
    }

    pub fn stop(&mut self) {
        if self.stopped { return; }
        println!("[Match:{}] Stopping.", self.name);
        for comp in self.components.iter_mut().rev() {
            comp.stop();
        }
        for dep in self.dependencies.iter_mut().rev() {
            dep.stop();
        }
        println!("[Match:{}] Stopped.", self.name);
        self.stopped = true;
    }
}

impl Drop for Match {
    fn drop(&mut self) {
        self.stop();
    }
}


pub struct Arena {
    pub name: String,
    pub matches: Vec<Match>,
    stopped: bool,
}

impl Arena {
    pub fn new(name: String, matches: Vec<Match>) -> Self {
        Arena { name, matches, stopped: false }
    }

    pub fn start_matches(&self) {
        println!("[ARENA:{}] Starting.", self.name);
        for m in self.matches.iter() {
            m.start();
        }
        println!("[ARENA{}] Started.", self.name);
    }

    pub fn stop_matches(&mut self) {
        if self.stopped { return; }
        println!("[ARENA:{}] Stopping matches.", self.name);
        for m in self.matches.iter_mut().rev() {
            m.stop();
        }
        println!("[ARENA:{}] Stopped matches.", self.name);
        self.stopped = true;
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        self.stop_matches();
    }
}

fn main() {
    let mut db_parent = Dependency::Database(DatabaseDependency {
        name: "DB_Prod_Master".to_string(),
        dependencies: vec![],
        stopped: false,
    });

    let db_logger_child = Dependency::Database(DatabaseDependency {
        name: "DB_Logger".to_string(),
        dependencies: vec![],
        stopped: false,
    });

    let db_cache_child = Dependency::Database(DatabaseDependency {
        name: "DB_Cache".to_string(),
        dependencies: vec![],
        stopped: false,
    });

    db_parent.add_child(db_logger_child);
    db_parent.add_child(db_cache_child);

    let deps1: Vec<Dependency> = vec![db_parent];

    let comps1: Vec<Component> = vec![
        Component::Application(ApplicationComponent { endpoint: "web app".to_string(), stopped: false }),
    ];
    let match1 = Match::new("end too end happy path", deps1, comps1);

    let matches: Vec<Match> = vec![match1];
    let arena = Arena::new(String::from("Component Test Suite"), matches);
    arena.start_matches();
}
