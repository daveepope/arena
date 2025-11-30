use std::ops::Drop;
use super::arena_match::ArenaMatch;

pub struct Arena {
    pub name: String,
    pub encounters: Vec<ArenaMatch>,
    running: bool
}

impl Arena {
    pub fn new(name: String, encounters: Vec<ArenaMatch>) -> Self {
        Arena { name, encounters, running: false }
    }

    pub fn commence(&mut self) {
        if self.running { return; }
        println!("[ARENA:{}] Starting.", self.name);
        for m in self.encounters.iter_mut() {
            m.start();
        }
        self.running = true;
        println!("[ARENA{}] Started.", self.name);
    }

    pub fn conclude(&mut self) {
        if !self.running { return;}
        println!("[ARENA:{}] Stopping matches.", self.name);
        for m in self.encounters.iter_mut().rev() {
            m.stop();
        }
        println!("[ARENA:{}] Stopped matches.", self.name);
        self.running = false;
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        self.conclude();
    }
}
