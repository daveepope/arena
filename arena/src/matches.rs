use super::component::{component_state, Component};
use super::dependency::{dependency_state, Dependency};
use super::dependency::RunnableDependency;
use super::playbook::{ActivePlaybook, Playbook};
use crate::lifecycle::message;
use crate::lifecycle::{
    panic_message, ArenaLifecycleState, ComponentState, DependencyState, Fault, LifecycleContext,
    Subject,
};
use async_trait::async_trait;
use futures::future::join_all;
use futures::FutureExt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;
use tracing::Instrument;

pub(crate) fn dependency_span(id: &str) -> tracing::Span {
    tracing::info_span!("subject", subject_kind = "dependency", subject_id = %id)
}

pub(crate) fn component_span(id: &str) -> tracing::Span {
    tracing::info_span!("subject", subject_kind = "component", subject_id = %id)
}

pub(crate) fn playbook_span(id: &str) -> tracing::Span {
    tracing::info_span!("subject", subject_kind = "playbook", subject_id = %id)
}

fn find_dependency_mut<'a>(
    deps: &'a mut [Dependency],
    identifier: &str,
) -> Option<&'a mut dyn RunnableDependency> {
    for dep in deps.iter_mut() {
        if dep.identifier() == identifier {
            return Some(dep.as_mut());
        }
        if let Some(found) = find_dependency_mut(dep.children_mut(), identifier) {
            return Some(found);
        }
    }
    None
}

async fn graceful_stop_dependency(dep: &mut Dependency) -> Option<Fault> {
    if dep.state().is_inactive() {
        return None;
    }
    let span = dependency_span(dep.identifier());
    async move {
        match AssertUnwindSafe(dep.stop()).catch_unwind().await {
            Ok(Ok(())) => None,
            Ok(Err(fault)) => Some(fault),
            Err(payload) => {
                let panic_text = panic_message(payload.as_ref());
                tracing::error!(
                    dependency = %dep.identifier(),
                    panic_message = %panic_text,
                    phase = "dependency_stop_panic",
                    "dependency panicked while stopping"
                );
                Some(
                    Fault::dependency(dep.identifier(), message::stop_failed()).caused_by(
                        Fault::from_panic(dep.identifier(), Subject::Dependency, payload.as_ref()),
                    ),
                )
            }
        }
    }
    .instrument(span)
    .await
}

async fn graceful_stop_component(comp: &mut Component) -> Option<Fault> {
    if comp.state().is_inactive() {
        return None;
    }
    let span = component_span(comp.identifier());
    async move {
        match AssertUnwindSafe(comp.stop()).catch_unwind().await {
            Ok(Ok(())) => None,
            Ok(Err(fault)) => Some(fault),
            Err(payload) => {
                let panic_text = panic_message(payload.as_ref());
                tracing::error!(
                    component = %comp.identifier(),
                    panic_message = %panic_text,
                    phase = "component_stop_panic",
                    "component panicked while stopping"
                );
                Some(
                    Fault::component(comp.identifier(), message::stop_failed()).caused_by(
                        Fault::from_panic(comp.identifier(), Subject::Component, payload.as_ref()),
                    ),
                )
            }
        }
    }
    .instrument(span)
    .await
}

async fn force_stop_dependency(dep: &mut Dependency) {
    let span = dependency_span(dep.identifier());
    async move {
        let outcome = AssertUnwindSafe(dep.force_stop()).catch_unwind().await;
        if let Err(payload) = outcome {
            tracing::error!(
                dependency = %dep.identifier(),
                panic_message = %panic_message(payload.as_ref()),
                phase = "dependency_force_stop_panic",
                "dependency panicked while being forcibly stopped"
            );
        }
    }
    .instrument(span)
    .await
}

async fn force_stop_component(comp: &mut Component) {
    let span = component_span(comp.identifier());
    async move {
        let outcome = AssertUnwindSafe(comp.force_stop()).catch_unwind().await;
        if let Err(payload) = outcome {
            tracing::error!(
                component = %comp.identifier(),
                panic_message = %panic_message(payload.as_ref()),
                phase = "component_force_stop_panic",
                "component panicked while being forcibly stopped"
            );
        }
    }
    .instrument(span)
    .await
}

#[async_trait]
pub trait MatchTrait: Send + Sync {
    async fn start(&mut self, ctx: &LifecycleContext) -> Result<(), Vec<Fault>>;
    async fn stop(&mut self, ctx: &LifecycleContext) -> Result<(), Vec<Fault>>;

    async fn force_stop_all(&mut self) {}

    fn release_all(&mut self) {}

    fn dependency_states(&self) -> Vec<DependencyState> {
        Vec::new()
    }

    fn component_states(&self) -> Vec<ComponentState> {
        Vec::new()
    }

    fn dependency(&self, _identifier: &str) -> Option<&(dyn RunnableDependency + '_)> {
        None
    }

    fn dependency_mut(&mut self, _identifier: &str) -> Option<&mut (dyn RunnableDependency + '_)> {
        None
    }

    async fn run_playbook(&self, _identifier: &str) -> Option<Result<Box<dyn ActivePlaybook>, Fault>> {
        None
    }
}

pub struct Match {
    pub name: String,
    dependencies: Vec<Dependency>,
    components: Vec<Component>,
    playbooks: Vec<(Box<dyn Playbook>, bool)>,
    active_playbooks: Vec<Box<dyn ActivePlaybook>>,
    started: bool,
}

impl Match {
    pub fn new(name: &str, dependencies: Vec<Dependency>, components: Vec<Component>) -> Self {
        Match {
            name: name.to_string(),
            dependencies,
            components,
            playbooks: Vec::new(),
            active_playbooks: Vec::new(),
            started: false,
        }
    }

    pub fn register_playbook(
        mut self,
        playbook: Box<dyn Playbook>,
        exec_on_dependency_start: bool,
    ) -> Self {
        self.playbooks.push((playbook, exec_on_dependency_start));
        self
    }

    fn snapshot(&self) -> (Vec<DependencyState>, Vec<ComponentState>) {
        (self.dependency_states(), self.component_states())
    }

    fn transition(&self, ctx: &LifecycleContext, state: ArenaLifecycleState) {
        let (dependencies, components) = self.snapshot();
        ctx.transition(state, dependencies, components);
    }

    async fn start_dependencies(&mut self) -> Vec<Fault> {
        let dep_count = self.dependencies.len();
        if dep_count > 0 {
            tracing::info!(
                match_name = %self.name,
                dependency_count = dep_count,
                phase = "dependencies_start_begin",
                "starting dependencies"
            );
        }
        let sw_batch = Instant::now();
        let match_label = self.name.clone();
        let deps = std::mem::take(&mut self.dependencies);

        let outcomes = join_all(deps.into_iter().enumerate().map(|(i, mut dep)| {
            let match_label = match_label.clone();
            async move {
                let id = dep.identifier().to_string();
                let span = dependency_span(&id);
                async move {
                    let sw_one = Instant::now();
                    let outcome = AssertUnwindSafe(dep.start()).catch_unwind().await;
                    if matches!(outcome, Ok(Ok(()))) {
                        tracing::info!(
                            match_name = %match_label,
                            dependency = %id,
                            elapsed = ?sw_one.elapsed(),
                            phase = "dependency_start_complete",
                            "dependency started"
                        );
                    }
                    (i, id, dep, outcome)
                }
                .instrument(span)
                .await
            }
        }))
        .await;

        let mut faults = Vec::new();
        let mut started = Vec::with_capacity(dep_count);
        for (i, id, dep, outcome) in outcomes {
            match outcome {
                Ok(Ok(())) => {}
                Ok(Err(fault)) => faults.push(fault),
                Err(payload) => faults.push(
                    Fault::dependency(&id, message::start_failed()).caused_by(Fault::from_panic(
                        &id,
                        Subject::Dependency,
                        payload.as_ref(),
                    )),
                ),
            }
            started.push((i, dep));
        }

        started.sort_by_key(|(i, _)| *i);
        self.dependencies = started.into_iter().map(|(_, dep)| dep).collect();

        if dep_count > 0 && faults.is_empty() {
            tracing::info!(
                match_name = %self.name,
                elapsed = ?sw_batch.elapsed(),
                dependency_count = dep_count,
                phase = "dependencies_start_end",
                "dependencies started"
            );
        }
        faults
    }

    async fn run_startup_playbooks(&mut self) -> Vec<Fault> {
        let startup: Vec<&dyn Playbook> = self
            .playbooks
            .iter()
            .filter_map(|(pb, exec_on_start)| exec_on_start.then(|| pb.as_ref()))
            .collect();

        if startup.is_empty() {
            return Vec::new();
        }

        tracing::info!(
            match_name = %self.name,
            playbook_count = startup.len(),
            phase = "playbook_parallel_begin",
            "running playbooks in parallel"
        );
        let sw_batch = Instant::now();
        let deps_ref: &[Dependency] = &self.dependencies;
        let match_label = self.name.clone();

        let outcomes = join_all(startup.iter().map(|pb| {
            let id = pb.identifier().to_string();
            let match_label = match_label.clone();
            async move {
                let span = playbook_span(&id);
                async move {
                    let sw_one = Instant::now();
                    let outcome = AssertUnwindSafe(pb.run(deps_ref)).catch_unwind().await;
                    (id, match_label, sw_one, outcome)
                }
                .instrument(span)
                .await
            }
        }))
        .await;

        let mut faults = Vec::new();
        let mut actives = Vec::with_capacity(outcomes.len());
        for (id, match_label, sw_one, outcome) in outcomes {
            match outcome {
                Ok(Ok(active)) => {
                    tracing::info!(
                        match_name = %match_label,
                        playbook = %id,
                        elapsed = ?sw_one.elapsed(),
                        phase = "playbook_run_complete",
                        "playbook applied"
                    );
                    actives.push(active);
                }
                Ok(Err(fault)) => faults.push(fault),
                Err(payload) => faults.push(
                    Fault::playbook(&id, message::playbook_failed()).caused_by(Fault::from_panic(
                        &id,
                        Subject::Playbook,
                        payload.as_ref(),
                    )),
                ),
            }
        }

        self.active_playbooks = actives;

        if faults.is_empty() {
            tracing::info!(
                match_name = %self.name,
                elapsed = ?sw_batch.elapsed(),
                phase = "playbook_parallel_end",
                "playbooks complete"
            );
        }
        faults
    }

    async fn start_components(&mut self) -> Vec<Fault> {
        let comp_count = self.components.len();
        if comp_count > 0 {
            tracing::info!(
                match_name = %self.name,
                component_count = comp_count,
                phase = "components_start_begin",
                "starting components"
            );
        }
        let sw_batch = Instant::now();
        let match_label = self.name.clone();
        let comps = std::mem::take(&mut self.components);

        let outcomes = join_all(comps.into_iter().enumerate().map(|(i, mut comp)| {
            let match_label = match_label.clone();
            async move {
                let id = comp.identifier().to_string();
                let span = component_span(&id);
                async move {
                    let sw_one = Instant::now();
                    let outcome = AssertUnwindSafe(comp.start()).catch_unwind().await;
                    if matches!(outcome, Ok(Ok(()))) {
                        tracing::info!(
                            match_name = %match_label,
                            component = %id,
                            elapsed = ?sw_one.elapsed(),
                            phase = "component_start_complete",
                            "component started"
                        );
                    }
                    (i, id, comp, outcome)
                }
                .instrument(span)
                .await
            }
        }))
        .await;

        let mut faults = Vec::new();
        let mut started = Vec::with_capacity(comp_count);
        for (i, id, comp, outcome) in outcomes {
            match outcome {
                Ok(Ok(())) => {}
                Ok(Err(fault)) => faults.push(fault),
                Err(payload) => faults.push(
                    Fault::component(&id, message::start_failed()).caused_by(Fault::from_panic(
                        &id,
                        Subject::Component,
                        payload.as_ref(),
                    )),
                ),
            }
            started.push((i, comp));
        }

        started.sort_by_key(|(i, _)| *i);
        self.components = started.into_iter().map(|(_, comp)| comp).collect();

        if comp_count > 0 && faults.is_empty() {
            tracing::info!(
                match_name = %self.name,
                elapsed = ?sw_batch.elapsed(),
                component_count = comp_count,
                phase = "components_start_end",
                "components started"
            );
        }
        faults
    }

    async fn graceful_teardown(&mut self, ctx: &LifecycleContext) -> Vec<Fault> {
        let mut faults = Vec::new();
        self.transition(ctx, ArenaLifecycleState::ComponentsStopping);
        tracing::info!(
            match_name = %self.name,
            component_count = self.components.len(),
            phase = "components_stop_begin",
            "stopping components"
        );
        let sw_comps = Instant::now();
        for comp in self.components.iter_mut().rev() {
            if let Some(fault) = graceful_stop_component(comp).await {
                faults.push(fault);
            }
        }
        tracing::info!(
            match_name = %self.name,
            elapsed = ?sw_comps.elapsed(),
            phase = "components_stop_end",
            "components stopped"
        );
        self.transition(ctx, ArenaLifecycleState::ComponentsStopped);

        self.active_playbooks.clear();

        self.transition(ctx, ArenaLifecycleState::DependenciesStopping);
        tracing::info!(
            match_name = %self.name,
            dependency_count = self.dependencies.len(),
            phase = "dependencies_stop_begin",
            "stopping dependencies"
        );
        let sw_deps = Instant::now();
        for dep in self.dependencies.iter_mut().rev() {
            if let Some(fault) = graceful_stop_dependency(dep).await {
                faults.push(fault);
            }
        }
        tracing::info!(
            match_name = %self.name,
            elapsed = ?sw_deps.elapsed(),
            phase = "dependencies_stop_end",
            "dependencies stopped"
        );
        self.transition(ctx, ArenaLifecycleState::DependenciesStopped);
        faults
    }
}

#[async_trait]
impl MatchTrait for Match {
    async fn start(&mut self, ctx: &LifecycleContext) -> Result<(), Vec<Fault>> {
        if self.started {
            return Ok(());
        }

        tracing::info!(match_name = %self.name, phase = "start_begin", "starting");
        let sw = Instant::now();

        self.transition(ctx, ArenaLifecycleState::DependenciesStarting);
        let mut faults = self.start_dependencies().await;
        if !faults.is_empty() {
            faults.extend(self.graceful_teardown(ctx).await);
            return Err(faults);
        }
        self.transition(ctx, ArenaLifecycleState::DependenciesStarted);

        self.transition(ctx, ArenaLifecycleState::PlaybooksRunning);
        let mut faults = self.run_startup_playbooks().await;
        if !faults.is_empty() {
            faults.extend(self.graceful_teardown(ctx).await);
            return Err(faults);
        }
        self.transition(ctx, ArenaLifecycleState::PlaybooksComplete);

        self.transition(ctx, ArenaLifecycleState::ComponentsStarting);
        let mut faults = self.start_components().await;
        if !faults.is_empty() {
            faults.extend(self.graceful_teardown(ctx).await);
            return Err(faults);
        }
        self.transition(ctx, ArenaLifecycleState::ComponentsStarted);

        tracing::info!(
            match_name = %self.name,
            elapsed = ?sw.elapsed(),
            phase = "start_end",
            "started"
        );
        self.started = true;
        Ok(())
    }

    async fn stop(&mut self, ctx: &LifecycleContext) -> Result<(), Vec<Fault>> {
        tracing::info!(match_name = %self.name, phase = "stop_begin", "stopping");
        let sw = Instant::now();

        let faults = self.graceful_teardown(ctx).await;

        tracing::info!(
            match_name = %self.name,
            elapsed = ?sw.elapsed(),
            phase = "stop_end",
            "stopped"
        );
        self.started = false;

        if faults.is_empty() {
            Ok(())
        } else {
            Err(faults)
        }
    }

    async fn force_stop_all(&mut self) {
        for comp in self.components.iter_mut().rev() {
            force_stop_component(comp).await;
        }
        for dep in self.dependencies.iter_mut().rev() {
            force_stop_dependency(dep).await;
        }
    }

    fn release_all(&mut self) {
        for comp in self.components.iter_mut().rev() {
            let _ = catch_unwind(AssertUnwindSafe(|| comp.release()));
        }
        let _ = catch_unwind(AssertUnwindSafe(|| self.active_playbooks.clear()));
        for dep in self.dependencies.iter_mut().rev() {
            let _ = catch_unwind(AssertUnwindSafe(|| dep.release()));
        }
    }

    fn dependency_states(&self) -> Vec<DependencyState> {
        self.dependencies
            .iter()
            .map(|d| dependency_state(d.as_ref()))
            .collect()
    }

    fn component_states(&self) -> Vec<ComponentState> {
        self.components
            .iter()
            .map(|c| component_state(c.as_ref()))
            .collect()
    }

    fn dependency(&self, identifier: &str) -> Option<&(dyn RunnableDependency + '_)> {
        super::dependency::find_dependency(&self.dependencies, identifier)
    }

    fn dependency_mut(&mut self, identifier: &str) -> Option<&mut (dyn RunnableDependency + '_)> {
        find_dependency_mut(&mut self.dependencies, identifier)
    }

    async fn run_playbook(&self, identifier: &str) -> Option<Result<Box<dyn ActivePlaybook>, Fault>> {
        let pb = self
            .playbooks
            .iter()
            .find(|(p, _)| p.identifier() == identifier)?;
        let span = playbook_span(identifier);
        Some(pb.0.run(&self.dependencies).instrument(span).await)
    }
}
