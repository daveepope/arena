use std::ops::Drop;
use super::encounter::{EncounterTrait};

pub struct Arena {
    pub name: String,
    pub encounters: Vec<Box<dyn EncounterTrait>>,
    running: bool
}

impl Arena {
    pub fn new(name: String, encounters: Vec<Box<dyn EncounterTrait>>) -> Self {
        Arena { name, encounters, running: false }
    }

    pub fn commence(&mut self) {
        if self.running { return; }
        println!("[Arena-{}] starting.", self.name);
        for m in self.encounters.iter_mut() {
            m.start();
        }
        self.running = true;
        println!("[Arena-{}] all Encounters started.", self.name);
    }

    pub fn conclude(&mut self) {
        if !self.running { return;}
        println!("[Arena-{}] stopping Encounters.", self.name);
        for m in self.encounters.iter_mut().rev() {
            m.stop();
        }
        println!("[Arena-{}] all Encounters stopped.", self.name);
        self.running = false;
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        self.conclude();
    }
}

#[cfg(test)]
mod tests {

    use mockall::mock;
    use crate::{Arena, EncounterTrait};

    mock! {
        Encounter {}
        impl EncounterTrait for Encounter {
            fn start(&mut self);
            fn stop(&mut self);
        }
    }

    #[test]
    fn test_arena_calls_start_on_all_encounters() {
        let mut mock1 = MockEncounter::new();
        mock1.expect_start().times(1).returning(|| {});
        mock1.expect_stop().times(1).returning(|| {});

        let mut mock2 = MockEncounter::new();
        mock2.expect_start().times(1).returning(|| {});
        mock2.expect_stop().times(1).returning(|| {});

        let encounters: Vec<Box<dyn EncounterTrait>> = vec![Box::new(mock1), Box::new(mock2)];
        let mut arena = Arena::new("TestArena".to_string(), encounters);

        arena.commence();
    }
}