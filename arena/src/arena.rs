use std::ops::Drop;
use super::encounter::Encounter;

pub struct Arena {
    pub name: String,
    pub encounters: Vec<Encounter>,
    running: bool
}

impl Arena {
    pub fn new(name: String, encounters: Vec<Encounter>) -> Self {
        Arena { name, encounters, running: false }
    }

    pub fn commence(&mut self) {
        if self.running { return; }
        println!("[Arena-{}] starting.", self.name);
        for m in self.encounters.iter_mut() {
            m.start();
        }
        self.running = true;
        println!("[Arena-{}] all matches started.", self.name);
    }

    pub fn conclude(&mut self) {
        if !self.running { return;}
        println!("[Arena-{}] stopping encounters.", self.name);
        for m in self.encounters.iter_mut().rev() {
            m.stop();
        }
        println!("[Arena-{}] all encounters stopped.", self.name);
        self.running = false;
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        self.conclude();
    }
}
