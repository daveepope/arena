use crate::matches::MatchTrait;
use futures::executor::block_on;
use futures::future::join_all;
use futures::FutureExt;
use std::panic::{AssertUnwindSafe, resume_unwind};
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

        let outcomes = join_all(
            matches
                .into_iter()
                .enumerate()
                .map(|(i, mut m)| {
                    let arena_name = arena_name.clone();
                    async move {
                        let sw_one = Instant::now();
                        let outcome = AssertUnwindSafe(async {
                            m.start().await;
                            m
                        })
                        .catch_unwind()
                        .await;
                        (i, arena_name, sw_one, outcome)
                    }
                }),
        )
        .await;

        let mut open_panics = Vec::new();
        let mut started = Vec::with_capacity(outcomes.len());
        for (i, arena_name, sw_one, outcome) in outcomes {
            match outcome {
                Ok(m) => {
                    tracing::info!(
                        arena = %arena_name,
                        match_index = i,
                        elapsed = ?sw_one.elapsed(),
                        phase = "match_open_complete",
                        "match opened"
                    );
                    started.push((i, m));
                }
                Err(payload) => open_panics.push(payload),
            }
        }

        if !open_panics.is_empty() {
            started.sort_by_key(|(i, _)| *i);
            for (_, mut m) in started.drain(..) {
                m.stop().await;
            }
            resume_unwind(open_panics.into_iter().next().unwrap());
        }

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
