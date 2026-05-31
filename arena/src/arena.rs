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
        Self {
            name,
            matches,
        }
    }

    pub async fn open(mut self) -> OpenArena {
        tracing::info!(arena = %self.name, phase = "open_begin", "opening");
        let sw = Instant::now();

        let matches = std::mem::take(&mut self.matches);
        let arena_name = self.name.clone();

        let mut started = join_all(
            matches
                .into_iter()
                .enumerate()
                .map(|(i, mut m)| {
                    let arena_name = arena_name.clone();
                    async move {
                        let sw_one = Instant::now();
                        m.start().await;
                        tracing::info!(
                            arena = %arena_name,
                            match_index = i,
                            elapsed = ?sw_one.elapsed(),
                            phase = "match_open_complete",
                            "match opened"
                        );
                        (i, m)
                    }
                }),
        )
        .await;

        started.sort_by_key(|(i, _)| *i);
        let matches = started.into_iter().map(|(_, m)| m).collect();

        tracing::info!(
            arena = %self.name,
            elapsed = ?sw.elapsed(),
            phase = "open_end",
            "open complete"
        );

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

    pub async fn run_playbook(
        &self,
        identifier: &str,
    ) -> Option<Box<dyn crate::playbook::ActivePlaybook>> {
        for m in &self.matches {
            if let Some(active) = m.run_playbook(identifier).await {
                return Some(active);
            }
        }
        None
    }

    pub async fn close(mut self) -> ClosedArena {
        self.internal_close().await;

        let name = std::mem::take(&mut self.name);
        let matches = std::mem::take(&mut self.matches);

        ClosedArena {
            name,
            matches,
        }
    }

    async fn internal_close(&mut self) {
        if !self.closed {
            tracing::info!(arena = %self.name, phase = "close_begin", "closing");
            let sw = Instant::now();

            let arena_name = self.name.clone();
            let matches = std::mem::take(&mut self.matches);

            let mut stopped = join_all(matches.into_iter().enumerate().map(
                |(i, mut m)| {
                    let arena_name = arena_name.clone();
                    async move {
                        let sw_one = Instant::now();
                        m.stop().await;
                        tracing::info!(
                            arena = %arena_name,
                            match_index = i,
                            elapsed = ?sw_one.elapsed(),
                            phase = "match_close_complete",
                            "match closed"
                        );
                        (i, m)
                    }
                },
            ))
            .await;

            stopped.sort_by_key(|(i, _)| *i);
            self.matches = stopped.into_iter().map(|(_, m)| m).collect();

            tracing::info!(
                arena = %self.name,
                elapsed = ?sw.elapsed(),
                phase = "close_end",
                "close complete"
            );

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
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let matches: Vec<Box<dyn MatchTrait>> = vec![Box::new(create_and_setup_stub_match())];

            let closed = ClosedArena::new("TestArena".to_string(), matches);
            let open = closed.open().await;

            drop(open);
        });
    }

    fn create_and_setup_stub_match() -> MockMatch {
        let mut stub_match = MockMatch::new();
        stub_match.expect_start().times(1).returning(|| ());
        stub_match.expect_stop().times(1).returning(|| ());
        stub_match
    }
}
