use std::ops::Drop;
use super::a_match::AMatch;

pub struct Arena {
    pub name: String,
    pub arena_matches: Vec<AMatch>,
    running: bool
}

impl Arena {
    pub fn new(name: String, arena_matches: Vec<AMatch>) -> Self {
        Arena { name, arena_matches, running: false }
    }

    pub fn commence(&mut self) {
        if self.running { return; }
        println!("[ARENA:{}] Starting.", self.name);
        for m in self.arena_matches.iter_mut() {
            m.start();
        }
        self.running = true;
        println!("[ARENA{}] Started.", self.name);
    }

    pub fn conclude(&mut self) {
        if !self.running { return;}
        println!("[ARENA:{}] Stopping matches.", self.name);
        for m in self.arena_matches.iter_mut().rev() {
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
