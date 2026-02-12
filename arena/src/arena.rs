use crate::encounter::EncounterTrait;
use futures::executor::block_on;
use std::time::Instant;

pub struct ClosedArena {
    pub name: String,
    pub encounters: Vec<Box<dyn EncounterTrait>>,
}

pub struct OpenArena {
    name: String,
    encounters: Vec<Box<dyn EncounterTrait>>,
    closed: bool,
}

impl ClosedArena {
    pub fn new(name: String, encounters: Vec<Box<dyn EncounterTrait>>) -> Self {
        Self { name, encounters }
    }

    pub async fn open(mut self) -> OpenArena {
        log::info!("[Arena-{}] opening.", self.name);
        let sw = Instant::now();

        for e in self.encounters.iter_mut() {
            e.start().await;
        }

        log::debug!(
            "[Arena-{}] open in {:?}.",
            self.name,
            sw.elapsed()
        );
        log::info!("[Arena-{}] opened.", self.name);

        OpenArena {
            name: self.name,
            encounters: self.encounters,
            closed: false,
        }
    }
}

impl OpenArena {
    pub fn dependency(
        &self,
        identifier: &str,
    ) -> Option<&(dyn crate::dependency::RunnableDependency + '_)> {
        for e in &self.encounters {
            if let Some(d) = e.dependency(identifier) {
                return Some(d);
            }
        }
        None
    }

    pub fn dependency_mut(
        &mut self,
        identifier: &str,
    ) -> Option<&mut (dyn crate::dependency::RunnableDependency + '_)> {
        for e in &mut self.encounters {
            if let Some(d) = e.dependency_mut(identifier) {
                return Some(d);
            }
        }
        None
    }

    pub async fn close(mut self) -> ClosedArena {
        self.internal_close().await;

        let name = std::mem::take(&mut self.name);
        let encounters = std::mem::take(&mut self.encounters);

        ClosedArena { name, encounters }
    }

    async fn internal_close(&mut self) {
        if !self.closed {
            log::info!("[Arena-{}] closing.", self.name);
            let sw = Instant::now();

            for e in self.encounters.iter_mut().rev() {
                e.stop().await;
            }

            log::debug!(
                "[Arena-{}] closed in {:?}.",
                self.name,
                sw.elapsed()
            );
            log::info!("[Arena-{}] closed.", self.name);

            self.closed = true;
        }
    }
}

impl Drop for OpenArena {
    fn drop(&mut self) {
        if !self.closed {
            block_on(self.internal_close());
        }
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
        let _ = env_logger::builder().is_test(true).try_init();
        let encounters: Vec<Box<dyn EncounterTrait>> = vec![
            Box::new(create_and_setup_stub_encounter()),
            Box::new(create_and_setup_stub_encounter()),
        ];

        let closed = ClosedArena::new("TestArena".to_string(), encounters);
        let open = closed.open().await;
        let _closed = open.close().await;
    }

    #[test]
    fn test_open_arena_auto_closes_on_drop() {
        let _ = env_logger::builder().is_test(true).try_init();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let encounters: Vec<Box<dyn EncounterTrait>> =
                vec![Box::new(create_and_setup_stub_encounter())];

            let closed = ClosedArena::new("TestArena".to_string(), encounters);
            let open = closed.open().await;

            drop(open);
        });
    }

    fn create_and_setup_stub_encounter() -> MockEncounter {
        let mut mock_encounter = MockEncounter::new();
        mock_encounter.expect_start().times(1).returning(|| ());
        mock_encounter.expect_stop().times(1).returning(|| ());
        mock_encounter
    }
}