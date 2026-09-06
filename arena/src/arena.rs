use crate::lifecycle::{
    panic_message, ArenaLifecycleState, ArenaLifecycleObserver, ArenaState, ComponentState,
    DependencyState, Fault, LifecycleContext, RunnableState,
};
use crate::matches::MatchTrait;
use futures::future::join_all;
use futures::FutureExt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Instant;

type Matches = Vec<Box<dyn MatchTrait>>;
type Observers = Vec<Arc<dyn ArenaLifecycleObserver>>;

pub struct ClosedArena {
    pub id: String,
    pub matches: Matches,
    observers: Observers,
}

pub struct OpenArena {
    id: String,
    matches: Matches,
    observers: Observers,
    context: Arc<LifecycleContext>,
    closed: bool,
}

fn collect_states(matches: &[Box<dyn MatchTrait>]) -> (Vec<DependencyState>, Vec<ComponentState>) {
    let mut dependencies = Vec::new();
    let mut components = Vec::new();
    for a_match in matches {
        dependencies.extend(a_match.dependency_states());
        components.extend(a_match.component_states());
    }
    (dependencies, components)
}

fn emit(
    context: &LifecycleContext,
    state: ArenaLifecycleState,
    matches: &[Box<dyn MatchTrait>],
) -> ArenaState {
    let (dependencies, components) = collect_states(matches);
    context.transition(state, dependencies, components)
}

fn finish(context: &LifecycleContext, matches: &[Box<dyn MatchTrait>]) -> ArenaState {
    let (dependencies, components) = collect_states(matches);
    context.finish(dependencies, components)
}

async fn forced_teardown(context: &LifecycleContext, matches: &mut Matches) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        emit(context, ArenaLifecycleState::ArenaTeardown, matches)
    }));

    for a_match in matches.iter_mut() {
        let outcome = AssertUnwindSafe(a_match.force_stop_all())
            .catch_unwind()
            .await;
        if let Err(payload) = outcome {
            context.record(Fault::arena(
                context.arena_id(),
                format!(
                    "match panicked during forced teardown: {}",
                    panic_message(payload.as_ref())
                ),
            ));
        }
    }

    let (dependencies, components) = collect_states(matches);
    for dependency in &dependencies {
        record_unexplained(context, dependency.into());
    }
    for component in &components {
        record_unexplained(context, component.into());
    }
}

struct Unexplained<'a> {
    subject: &'a str,
    id: &'a str,
    state: RunnableState,
    explained: bool,
    dependencies: Vec<Unexplained<'a>>,
}

impl<'a> From<&'a DependencyState> for Unexplained<'a> {
    fn from(value: &'a DependencyState) -> Self {
        Unexplained {
            subject: "dependency",
            id: &value.id,
            state: value.state,
            explained: !value.faults.is_empty(),
            dependencies: value.children.iter().map(Unexplained::from).collect(),
        }
    }
}

impl<'a> From<&'a ComponentState> for Unexplained<'a> {
    fn from(value: &'a ComponentState) -> Self {
        Unexplained {
            subject: "component",
            id: &value.id,
            state: value.state,
            explained: !value.faults.is_empty(),
            dependencies: value.children.iter().map(Unexplained::from).collect(),
        }
    }
}

fn record_unexplained(context: &LifecycleContext, node: Unexplained<'_>) {
    if !node.state.is_inactive() && !node.explained {
        context.record(Fault::arena(
            context.arena_id(),
            format!(
                "{} '{}' is {} after forced teardown and reported no fault",
                node.subject, node.id, node.state
            ),
        ));
    }
    for child in node.dependencies {
        record_unexplained(context, child);
    }
}

impl std::fmt::Debug for ClosedArena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClosedArena")
            .field("id", &self.id)
            .field("matches", &self.matches.len())
            .finish()
    }
}

impl std::fmt::Debug for OpenArena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenArena")
            .field("id", &self.id)
            .field("state", &self.context.current())
            .field("matches", &self.matches.len())
            .finish()
    }
}

impl ClosedArena {
    pub fn new(id: String, matches: Matches) -> Self {
        Self {
            id,
            matches,
            observers: Vec::new(),
        }
    }

    pub fn observe(mut self, observer: Arc<dyn ArenaLifecycleObserver>) -> Self {
        self.observers.push(observer);
        self
    }

    pub fn state(&self) -> ArenaState {
        let (dependencies, components) = collect_states(&self.matches);
        ArenaState::new(
            self.id.clone(),
            ArenaLifecycleState::ArenaCreated,
            dependencies,
            components,
            Vec::new(),
        )
    }

    pub async fn open(mut self) -> Result<OpenArena, ArenaState> {
        tracing::info!(arena = %self.id, phase = "open_begin", "opening");
        let sw = Instant::now();

        let context = Arc::new(LifecycleContext::new(
            self.id.clone(),
            self.observers.clone(),
        ));
        let mut matches = std::mem::take(&mut self.matches);
        emit(&context, ArenaLifecycleState::ArenaStarting, &matches);

        let arena_id = self.id.clone();
        let outcomes = join_all(matches.into_iter().enumerate().map(|(i, mut m)| {
            let arena_id = arena_id.clone();
            let context = Arc::clone(&context);
            async move {
                let sw_one = Instant::now();
                let outcome = AssertUnwindSafe(m.start(&context)).catch_unwind().await;
                (i, arena_id, sw_one, m, outcome)
            }
        }))
        .await;

        let mut faults = Vec::new();
        let mut started = Vec::with_capacity(outcomes.len());
        for (i, arena_id, sw_one, m, outcome) in outcomes {
            match outcome {
                Ok(Ok(())) => tracing::info!(
                    arena = %arena_id,
                    match_index = i,
                    elapsed = ?sw_one.elapsed(),
                    phase = "match_open_complete",
                    "match opened"
                ),
                Ok(Err(match_faults)) => faults.extend(match_faults),
                Err(payload) => faults.push(Fault::arena(
                    &arena_id,
                    format!(
                        "match {i} panicked while starting: {}",
                        panic_message(payload.as_ref())
                    ),
                )),
            }
            started.push((i, m));
        }

        started.sort_by_key(|(i, _)| *i);
        matches = started.into_iter().map(|(_, m)| m).collect();

        if !faults.is_empty() {
            for fault in faults {
                context.record(fault);
            }
            forced_teardown(&context, &mut matches).await;
            let state = finish(&context, &matches);
            tracing::error!(
                arena = %self.id,
                elapsed = ?sw.elapsed(),
                phase = "open_faulted",
                "open faulted"
            );
            return Err(state);
        }

        emit(&context, ArenaLifecycleState::ArenaOpen, &matches);
        tracing::info!(
            arena = %self.id,
            elapsed = ?sw.elapsed(),
            phase = "open_end",
            "open complete"
        );

        Ok(OpenArena {
            id: self.id,
            matches,
            observers: self.observers,
            context,
            closed: false,
        })
    }
}

impl OpenArena {
    pub fn state(&self) -> ArenaState {
        let (dependencies, components) = collect_states(&self.matches);
        ArenaState::new(
            self.id.clone(),
            self.context.current(),
            dependencies,
            components,
            self.context.recorded_faults(),
        )
    }

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
    ) -> Option<Result<Box<dyn crate::playbook::ActivePlaybook>, Fault>> {
        for m in &self.matches {
            if let Some(active) = m.run_playbook(identifier).await {
                return Some(active);
            }
        }
        None
    }

    pub async fn close(mut self) -> Result<ClosedArena, ArenaState> {
        let state = self.internal_close().await;

        let id = std::mem::take(&mut self.id);
        let matches = std::mem::take(&mut self.matches);
        let observers = std::mem::take(&mut self.observers);

        if state.state == ArenaLifecycleState::ArenaFaulted {
            return Err(state);
        }

        Ok(ClosedArena {
            id,
            matches,
            observers,
        })
    }

    async fn internal_close(&mut self) -> ArenaState {
        if self.closed {
            return self.state();
        }

        tracing::info!(arena = %self.id, phase = "close_begin", "closing");
        let sw = Instant::now();

        let context = Arc::clone(&self.context);
        let _ = catch_unwind(AssertUnwindSafe(|| {
            emit(&context, ArenaLifecycleState::ArenaClosing, &self.matches)
        }));

        for (i, m) in self.matches.iter_mut().enumerate() {
            let sw_one = Instant::now();
            let outcome = AssertUnwindSafe(m.stop(&context)).catch_unwind().await;
            match outcome {
                Ok(Ok(())) => tracing::info!(
                    arena = %self.id,
                    match_index = i,
                    elapsed = ?sw_one.elapsed(),
                    phase = "match_close_complete",
                    "match closed"
                ),
                Ok(Err(match_faults)) => {
                    for fault in match_faults {
                        context.record(fault);
                    }
                }
                Err(payload) => context.record(Fault::arena(
                    &self.id,
                    format!(
                        "match {i} panicked while stopping: {}",
                        panic_message(payload.as_ref())
                    ),
                )),
            }
        }

        forced_teardown(&context, &mut self.matches).await;
        let state = finish(&context, &self.matches);
        self.closed = true;

        tracing::info!(
            arena = %self.id,
            elapsed = ?sw.elapsed(),
            terminal_state = %state.state,
            phase = "close_end",
            "close complete"
        );

        state
    }
}

impl Drop for OpenArena {
    fn drop(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        tracing::warn!(
            arena = %self.id,
            phase = "drop_without_close",
            "arena dropped without close; releasing subjects without a graceful stop"
        );
        for a_match in self.matches.iter_mut() {
            let outcome = catch_unwind(AssertUnwindSafe(|| a_match.release_all()));
            if let Err(payload) = outcome {
                tracing::error!(
                    arena = %self.id,
                    panic_message = %panic_message(payload.as_ref()),
                    phase = "drop_release_panic",
                    "panic while releasing arena subjects during drop"
                );
            }
        }
    }
}
