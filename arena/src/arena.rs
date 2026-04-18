use crate::matches::MatchTrait;
use futures::executor::block_on;
use futures::future::join_all;
use std::time::Instant;

pub struct ClosedArena {
    pub name: String,
    pub matches: Vec<Box<dyn MatchTrait>>,
}

pub struct OpenArena {
    name: String,
    matches: Vec<Box<dyn MatchTrait>>,
    closed: bool,
}

impl ClosedArena {
    pub fn new(name: String, matches: Vec<Box<dyn MatchTrait>>) -> Self {
        Self { name, matches }
    }

    pub async fn open(mut self) -> OpenArena {
        log::info!("[Arena-{}] opening.", self.name);
        let sw = Instant::now();

        let matches = std::mem::take(&mut self.matches);

        let mut started = join_all(matches.into_iter().enumerate().map(|(i, mut m)| async move {
            m.start().await;
            (i, m)
        }))
        .await;

        started.sort_by_key(|(i, _)| *i);
        let matches = started.into_iter().map(|(_, m)| m).collect();

        log::debug!(
            "[Arena-{}] open in {:?}.",
            self.name,
            sw.elapsed()
        );
        log::info!("[Arena-{}] opened.", self.name);

        OpenArena {
            name: self.name,
            matches,
            closed: false,
        }
    }
}

impl OpenArena {
    pub fn dependency(
        &self,
        identifier: &str,
    ) -> Option<&(dyn crate::dependency::RunnableDependency + '_)> {
        for m in &self.matches {
            if let Some(d) = m.dependency(identifier) {
                return Some(d);
            }
        }
        None
    }

    pub fn dependency_mut(
        &mut self,
        identifier: &str,
    ) -> Option<&mut (dyn crate::dependency::RunnableDependency + '_)> {
        for m in &mut self.matches {
            if let Some(d) = m.dependency_mut(identifier) {
                return Some(d);
            }
        }
        None
    }

    pub async fn close(mut self) -> ClosedArena {
        self.internal_close().await;

        let name = std::mem::take(&mut self.name);
        let matches = std::mem::take(&mut self.matches);

        ClosedArena { name, matches }
    }

    async fn internal_close(&mut self) {
        if !self.closed {
            log::info!("[Arena-{}] closing.", self.name);
            let sw = Instant::now();

            let matches = std::mem::take(&mut self.matches);

            let mut stopped = join_all(matches.into_iter().enumerate().map(|(i, mut m)| async move {
                m.stop().await;
                (i, m)
            }))
            .await;

            stopped.sort_by_key(|(i, _)| *i);
            self.matches = stopped.into_iter().map(|(_, m)| m).collect();

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
        Match {}
        #[async_trait]
        impl MatchTrait for Match {
            async fn start(&mut self);
            async fn stop(&mut self);
        }
    }

    #[tokio::test]
    async fn test_arena_calls_start_and_stop_on_all_matches() {
        let _ = env_logger::builder().is_test(true).try_init();
        let matches: Vec<Box<dyn MatchTrait>> = vec![
            Box::new(create_and_setup_stub_match()),
            Box::new(create_and_setup_stub_match()),
        ];

        let closed = ClosedArena::new("TestArena".to_string(), matches);
        let open = closed.open().await;
        let _closed = open.close().await;
    }

    #[test]
    fn test_open_arena_auto_closes_on_drop() {
        let _ = env_logger::builder().is_test(true).try_init();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let matches: Vec<Box<dyn MatchTrait>> =
                vec![Box::new(create_and_setup_stub_match())];

            let closed = ClosedArena::new("TestArena".to_string(), matches);
            let open = closed.open().await;

            drop(open);
        });
    }

    fn create_and_setup_stub_match() -> MockMatch {
        let mut mock_match = MockMatch::new();
        mock_match.expect_start().times(1).returning(|| ());
        mock_match.expect_stop().times(1).returning(|| ());
        mock_match
    }
}
