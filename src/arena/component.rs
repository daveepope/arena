use std::ops::Drop;

pub enum Component {
    Application(ManagedProcessComponent),
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

pub struct ManagedProcessComponent {
    endpoint: String,
    stopped: bool,
}

impl ManagedProcessComponent {
    pub fn new(endpoint: String) -> Self {
        ManagedProcessComponent { endpoint, stopped: false }
    }

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

impl Drop for ManagedProcessComponent {
    fn drop(&mut self) {
        self.stop();
    }
}
