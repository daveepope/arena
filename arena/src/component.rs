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
        println!("[Component-{}] starting.", self.endpoint);
        println!("[Component-{}] started.", self.endpoint);
    }

    pub fn stop(&mut self) {
        if self.stopped { return; }
        println!("[Component-{}] stopping.", self.endpoint);
        println!("[Component-{}] stopped.", self.endpoint);
        self.stopped = true;
    }
}

impl Drop for ManagedProcessComponent {
    fn drop(&mut self) {
        self.stop();
    }
}
