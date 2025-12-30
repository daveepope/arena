use crate::encounter::EncounterTrait;

pub struct Arena {
    pub name: String,
    pub encounters: Vec<Box<dyn EncounterTrait>>,
    running: bool,
}

impl Arena {
    pub fn new(name: String, encounters: Vec<Box<dyn EncounterTrait>>) -> Self {
        Arena { name, encounters, running: false }
    }

    pub async fn commence(&mut self) {
        if self.running {
            return;
        }

        println!("[Arena-{}] starting.", self.name);

        for m in self.encounters.iter_mut() {
            m.start().await;
        }

        self.running = true;
        println!("[Arena-{}] all Encounters started.", self.name);
    }

    pub async fn conclude(&mut self) {
        if !self.running {
            return;
        }

        println!("[Arena-{}] stopping Encounters.", self.name);

        for m in self.encounters.iter_mut().rev() {
            m.stop().await;
        }

        println!("[Arena-{}] all Encounters stopped.", self.name);
        self.running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;

    mock! {
        Encounter {}
        #[async_trait]
        impl EncounterTrait for Encounter {
            async fn start(&mut self);
            async fn stop(&mut self);
        }
    }

    #[tokio::test]
    async fn test_arena_calls_start_and_stop_on_all_encounters() {
        let encounters: Vec<Box<dyn EncounterTrait>> = vec![
            Box::new(create_and_setup_stub_encounter()),
            Box::new(create_and_setup_stub_encounter()),
        ];
        let mut arena = Arena::new("TestArena".to_string(), encounters);

        arena.commence().await;
        arena.conclude().await;
    }

    fn create_and_setup_stub_encounter() -> MockEncounter {
        let mut mock_encounter = MockEncounter::new();
        mock_encounter
            .expect_start()
            .times(1)
            .returning(|| Box::pin(async {}));
        mock_encounter
            .expect_stop()
            .times(1)
            .returning(|| Box::pin(async {}));
        mock_encounter
    }
}