use super::component::{component_state, Component};
use super::dependency::{dependency_state, Dependency};
use super::dependency::RunnableDependency;
use super::playbook::lifecycle::{PlaybookKind, PlaybookEvent};
use super::playbook::{ActivePlaybook, Playbook};
use crate::lifecycle::bus::Publisher;
use std::sync::{Arc, OnceLock};
use crate::lifecycle::message;
use crate::lifecycle::{
    panic_message, ArenaLifecycleState, ComponentState, DependencyState, Fault, ArenaLifecyclePublisher,
    MatchSlot, Subject,
};
use async_trait::async_trait;
use futures::future::join_all;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;
use tracing::Instrument;

pub const SUBJECT_KIND_DEPENDENCY: &str = "dependency";
pub const SUBJECT_KIND_COMPONENT: &str = "component";
pub const SUBJECT_KIND_PLAYBOOK: &str = "playbook";

pub fn dependency_span(id: &str) -> tracing::Span {
    tracing::info_span!(
        "subject",
        arena.subject.kind = SUBJECT_KIND_DEPENDENCY,
        arena.subject.id = %id
    )
}

pub fn component_span(id: &str) -> tracing::Span {
    tracing::info_span!(
        "subject",
        arena.subject.kind = SUBJECT_KIND_COMPONENT,
        arena.subject.id = %id
    )
}

pub(crate) fn playbook_span(id: &str) -> tracing::Span {
    tracing::info_span!(
        "subject",
        arena.subject.kind = SUBJECT_KIND_PLAYBOOK,
        arena.subject.id = %id
    )
}

fn fault_reason(fault: &Fault) -> String {
    let mut reason = fault.message.clone();
    for nested in fault.flatten().into_iter().skip(1) {
        reason.push_str(" | caused by ");
        reason.push_str(&nested.message);
    }
    reason
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
            Ok(Err(fault)) => {
                tracing::error!(
                    dependency = %dep.identifier(),
                    reason = %fault_reason(&fault),
                    phase = "dependency_stop_faulted",
                    "dependency failed to stop"
                );
                Some(fault)
            }
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
            Ok(Err(fault)) => {
                tracing::error!(
                    component = %comp.identifier(),
                    reason = %fault_reason(&fault),
                    phase = "component_stop_faulted",
                    "component failed to stop"
                );
                Some(fault)
            }
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

async fn force_stop_dependency(dep: &mut Dependency) -> Option<Fault> {
    let id = dep.identifier().to_string();
    let span = dependency_span(&id);
    async move {
        let outcome = AssertUnwindSafe(dep.force_stop()).catch_unwind().await;
        match outcome {
            Ok(()) => None,
            Err(payload) => {
                tracing::error!(
                    dependency = %id,
                    panic_message = %panic_message(payload.as_ref()),
                    phase = "dependency_force_stop_panic",
                    "dependency panicked while being forcibly stopped"
                );
                Some(
                    Fault::dependency(&id, message::stop_failed()).caused_by(Fault::from_panic(
                        &id,
                        Subject::Dependency,
                        payload.as_ref(),
                    )),
                )
            }
        }
    }
    .instrument(span)
    .await
}

async fn force_stop_component(comp: &mut Component) -> Option<Fault> {
    let id = comp.identifier().to_string();
    let span = component_span(&id);
    async move {
        let outcome = AssertUnwindSafe(comp.force_stop()).catch_unwind().await;
        match outcome {
            Ok(()) => None,
            Err(payload) => {
                tracing::error!(
                    component = %id,
                    panic_message = %panic_message(payload.as_ref()),
                    phase = "component_force_stop_panic",
                    "component panicked while being forcibly stopped"
                );
                Some(
                    Fault::component(&id, message::stop_failed()).caused_by(Fault::from_panic(
                        &id,
                        Subject::Component,
                        payload.as_ref(),
                    )),
                )
            }
        }
    }
    .instrument(span)
    .await
}

#[async_trait]
pub trait MatchTrait: Send + Sync {
    async fn start(&mut self, ctx: &ArenaLifecyclePublisher) -> Result<(), Vec<Fault>>;


    async fn stop(&mut self, ctx: &ArenaLifecyclePublisher) -> Result<(), Vec<Fault>>;

    fn attach_report_slot(&mut self, _slot: MatchSlot) {}

    fn report_slot(&self) -> Option<MatchSlot> {
        None
    }

    async fn force_stop_all(&mut self) -> Vec<Fault> {
        Vec::new()
    }

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

    fn attach_playbook_publisher(
        &self,
        _arena_id: &str,
        _publisher: Arc<dyn Publisher<PlaybookEvent>>,
    ) {
    }
}

pub(crate) fn matching_component_ids(components: &[Component], requested: &str) -> Vec<String> {
    let mut found = Vec::new();
    collect_matching_component_ids(components, requested, &mut found);
    found
}

fn collect_matching_component_ids(
    components: &[Component],
    requested: &str,
    found: &mut Vec<String>,
) {
    let infix = format!("-{requested}-");
    for component in components {
        let identifier = component.identifier();
        if identifier == requested || identifier.contains(&infix) {
            found.push(identifier.to_string());
        }
        collect_matching_component_ids(component.children(), requested, found);
    }
}

fn find_component_mut<'a>(components: &'a mut [Component], identifier: &str) -> Option<&'a mut Component> {
    for index in 0..components.len() {
        if components[index].identifier() == identifier {
            return Some(&mut components[index]);
        }
    }
    for component in components.iter_mut() {
        if let Some(found) = find_component_mut(component.children_mut(), identifier) {
            return Some(found);
        }
    }
    None
}

pub struct Match {
    pub name: String,
    dependencies: Vec<Dependency>,
    components: Vec<Component>,
    playbooks: Vec<(Box<dyn Playbook>, bool)>,
    active_playbooks: Vec<Box<dyn ActivePlaybook>>,
    cpu_profiled_component_ids: Vec<String>,
    cpu_profile_limit_bytes: Option<usize>,
    slot: OnceLock<MatchSlot>,
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
            cpu_profiled_component_ids: Vec::new(),
            cpu_profile_limit_bytes: None,
            slot: OnceLock::new(),
            started: false,
        }
    }

    pub fn with_cpu_profiled_components(mut self, component_ids: Vec<String>) -> Self {
        self.cpu_profiled_component_ids = component_ids;
        self
    }

    pub fn with_cpu_profile_limit_bytes(mut self, limit_bytes: usize) -> Self {
        self.cpu_profile_limit_bytes = Some(limit_bytes);
        self
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

    fn slot(&self, ctx: &ArenaLifecyclePublisher) -> MatchSlot {
        *self.slot.get_or_init(|| ctx.reserve_match())
    }

    fn transition(&self, ctx: &ArenaLifecyclePublisher, state: ArenaLifecycleState) {
        let (dependencies, components) = self.snapshot();
        ctx.report_match(self.slot(ctx), dependencies, components);
        let published = ctx.transition(state);
        if published.state != state {
            ctx.notify_progress();
        }
    }

    fn report_progress(
        &self,
        ctx: &ArenaLifecyclePublisher,
        dependencies: Vec<DependencyState>,
        components: Vec<ComponentState>,
    ) {
        ctx.report_match(self.slot(ctx), dependencies, components);
        ctx.notify_progress();
    }

    async fn start_dependencies(&mut self, ctx: &ArenaLifecyclePublisher) -> Vec<Fault> {
        let dep_count = self.dependencies.len();
        if dep_count > 0 {
            tracing::debug!(
                match_name = %self.name,
                dependency_count = dep_count,
                phase = "dependencies_start_begin",
                "starting dependencies"
            );
        }
        let sw_batch = Instant::now();
        let match_label = self.name.clone();
        let deps = std::mem::take(&mut self.dependencies);
        let pending_states: Vec<DependencyState> =
            deps.iter().map(|d| dependency_state(d.as_ref())).collect();

        let mut in_flight: FuturesUnordered<_> = deps
            .into_iter()
            .enumerate()
            .map(|(i, mut dep)| {
                let match_label = match_label.clone();
                async move {
                    let id = dep.identifier().to_string();
                    let span = dependency_span(&id);
                    async move {
                        let sw_one = Instant::now();
                        let outcome = AssertUnwindSafe(dep.start()).catch_unwind().await;
                        match &outcome {
                            Ok(Err(fault)) => tracing::error!(
                                match_name = %match_label,
                                dependency = %id,
                                reason = %fault_reason(fault),
                                phase = "dependency_start_faulted",
                                "dependency failed to start"
                            ),
                            Err(payload) => tracing::error!(
                                match_name = %match_label,
                                dependency = %id,
                                panic_message = %panic_message(payload.as_ref()),
                                phase = "dependency_start_panic",
                                "dependency panicked while starting"
                            ),
                            Ok(Ok(())) => {}
                        }
                        if matches!(outcome, Ok(Ok(()))) {
                            tracing::debug!(
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
            })
            .collect();

        let mut faults = Vec::new();
        let mut slots: Vec<Option<Dependency>> = (0..dep_count).map(|_| None).collect();
        while let Some((i, id, dep, outcome)) = in_flight.next().await {
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
            slots[i] = Some(dep);

            let progress: Vec<DependencyState> = slots
                .iter()
                .enumerate()
                .map(|(j, slot)| match slot {
                    Some(dep) => dependency_state(dep.as_ref()),
                    None => pending_states[j].clone(),
                })
                .collect();
            self.report_progress(ctx, progress, self.component_states());
        }

        self.dependencies = slots
            .into_iter()
            .map(|slot| slot.expect("every dependency slot filled by its own future"))
            .collect();

        if dep_count > 0 && faults.is_empty() {
            tracing::debug!(
                match_name = %self.name,
                elapsed = ?sw_batch.elapsed(),
                dependency_count = dep_count,
                phase = "dependencies_start_end",
                "dependencies started"
            );
        }
        faults
    }

    fn report_playbook_fault(&self, identifier: &str, fault: Fault) {
        if let Some((playbook, _)) = self
            .playbooks
            .iter()
            .find(|(p, _)| p.identifier() == identifier)
        {
            playbook.lifecycle().report_fault(fault);
        }
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

        tracing::debug!(
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
                    match &outcome {
                        Ok(Err(fault)) => tracing::error!(
                            match_name = %match_label,
                            playbook = %id,
                            reason = %fault_reason(fault),
                            phase = "playbook_run_faulted",
                            "playbook failed to run"
                        ),
                        Err(payload) => tracing::error!(
                            match_name = %match_label,
                            playbook = %id,
                            panic_message = %panic_message(payload.as_ref()),
                            phase = "playbook_run_panic",
                            "playbook panicked while running"
                        ),
                        Ok(Ok(_)) => {}
                    }
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
                    tracing::debug!(
                        match_name = %match_label,
                        playbook = %id,
                        elapsed = ?sw_one.elapsed(),
                        phase = "playbook_run_complete",
                        "playbook applied"
                    );
                    actives.push(active);
                }
                Ok(Err(fault)) => faults.push(fault),
                Err(payload) => {
                    let fault = Fault::playbook(&id, message::playbook_failed()).caused_by(
                        Fault::from_panic(&id, Subject::Playbook, payload.as_ref()),
                    );
                    self.report_playbook_fault(&id, fault.clone());
                    faults.push(fault)
                }
            }
        }

        self.active_playbooks = actives;
        for (playbook, exec_on_start) in &self.playbooks {
            if *exec_on_start {
                playbook.lifecycle().release();
            }
        }

        if faults.is_empty() {
            tracing::debug!(
                match_name = %self.name,
                elapsed = ?sw_batch.elapsed(),
                phase = "playbook_parallel_end",
                "playbooks complete"
            );
        }
        faults
    }

    fn attach_cpu_profilers(&mut self) -> Vec<Fault> {
        let requested = std::mem::take(&mut self.cpu_profiled_component_ids);
        let mut faults = Vec::new();
        for name in requested {
            let matches = matching_component_ids(&self.components, &name);
            let resolved = match matches.as_slice() {
                [single] => single.clone(),
                [] => {
                    tracing::error!(
                        match_name = %self.name,
                        component = %name,
                        phase = "cpu_profiler_component_not_found",
                        "cpu profiler requested for a component that is not in this arena"
                    );
                    faults.push(Fault::component(
                        name.clone(),
                        format!("cpu profiler requested for '{name}', which is not a component of this arena"),
                    ));
                    continue;
                }
                several => {
                    tracing::error!(
                        match_name = %self.name,
                        component = %name,
                        phase = "cpu_profiler_component_ambiguous",
                        "cpu profiler request matched more than one component"
                    );
                    faults.push(Fault::component(
                        name.clone(),
                        format!(
                            "cpu profiler requested for '{name}', which matches {} components: {}",
                            several.len(),
                            several.join(", ")
                        ),
                    ));
                    continue;
                }
            };

            let Some(component) = find_component_mut(&mut self.components, &resolved) else {
                tracing::error!(
                    match_name = %self.name,
                    component = %resolved,
                    phase = "cpu_profiler_component_unreachable",
                    "cpu profiler could not reach a component that resolution had just found"
                );
                faults.push(Fault::component(
                    resolved.clone(),
                    format!("cpu profiler could not attach to '{resolved}'"),
                ));
                continue;
            };
            if let Err(fault) = component.attach_cpu_profiler(self.cpu_profile_limit_bytes) {
                tracing::error!(
                    match_name = %self.name,
                    component = %resolved,
                    reason = %fault_reason(&fault),
                    phase = "cpu_profiler_attach_faulted",
                    "cpu profiler could not be attached"
                );
                faults.push(fault);
            }
        }
        faults
    }

    async fn start_components(&mut self, ctx: &ArenaLifecyclePublisher) -> Vec<Fault> {
        let attach_faults = self.attach_cpu_profilers();
        if !attach_faults.is_empty() {
            return attach_faults;
        }
        let comp_count = self.components.len();
        if comp_count > 0 {
            tracing::debug!(
                match_name = %self.name,
                component_count = comp_count,
                phase = "components_start_begin",
                "starting components"
            );
        }
        let sw_batch = Instant::now();
        let match_label = self.name.clone();
        let comps = std::mem::take(&mut self.components);
        let pending_states: Vec<ComponentState> =
            comps.iter().map(|c| component_state(c.as_ref())).collect();

        let mut in_flight: FuturesUnordered<_> = comps
            .into_iter()
            .enumerate()
            .map(|(i, mut comp)| {
                let match_label = match_label.clone();
                async move {
                    let id = comp.identifier().to_string();
                    let span = component_span(&id);
                    async move {
                        let sw_one = Instant::now();
                        let outcome = AssertUnwindSafe(comp.start()).catch_unwind().await;
                        match &outcome {
                            Ok(Err(fault)) => tracing::error!(
                                match_name = %match_label,
                                component = %id,
                                reason = %fault_reason(fault),
                                phase = "component_start_faulted",
                                "component failed to start"
                            ),
                            Err(payload) => tracing::error!(
                                match_name = %match_label,
                                component = %id,
                                panic_message = %panic_message(payload.as_ref()),
                                phase = "component_start_panic",
                                "component panicked while starting"
                            ),
                            Ok(Ok(())) => {}
                        }
                        if matches!(outcome, Ok(Ok(()))) {
                            tracing::debug!(
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
            })
            .collect();

        let mut faults = Vec::new();
        let mut slots: Vec<Option<Component>> = (0..comp_count).map(|_| None).collect();
        while let Some((i, id, comp, outcome)) = in_flight.next().await {
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
            slots[i] = Some(comp);

            let progress: Vec<ComponentState> = slots
                .iter()
                .enumerate()
                .map(|(j, slot)| match slot {
                    Some(comp) => component_state(comp.as_ref()),
                    None => pending_states[j].clone(),
                })
                .collect();
            self.report_progress(ctx, self.dependency_states(), progress);
        }

        self.components = slots
            .into_iter()
            .map(|slot| slot.expect("every component slot filled by its own future"))
            .collect();

        if comp_count > 0 && faults.is_empty() {
            tracing::debug!(
                match_name = %self.name,
                elapsed = ?sw_batch.elapsed(),
                component_count = comp_count,
                phase = "components_start_end",
                "components started"
            );
        }
        faults
    }

    async fn graceful_teardown(&mut self, ctx: &ArenaLifecyclePublisher) -> Vec<Fault> {
        let mut faults = Vec::new();
        self.transition(ctx, ArenaLifecycleState::ComponentsStopping);
        tracing::debug!(
            match_name = %self.name,
            component_count = self.components.len(),
            phase = "components_stop_begin",
            "stopping components"
        );
        let sw_comps = Instant::now();
        let comps = std::mem::take(&mut self.components);
        let comp_count = comps.len();
        let pending_states: Vec<ComponentState> =
            comps.iter().map(|c| component_state(c.as_ref())).collect();
        let mut in_flight: FuturesUnordered<_> = comps
            .into_iter()
            .enumerate()
            .map(|(i, mut comp)| async move {
                let fault = graceful_stop_component(&mut comp).await;
                (i, comp, fault)
            })
            .collect();
        let mut slots: Vec<Option<Component>> = (0..comp_count).map(|_| None).collect();
        while let Some((i, comp, fault)) = in_flight.next().await {
            if let Some(fault) = fault {
                faults.push(fault);
            }
            slots[i] = Some(comp);

            let progress: Vec<ComponentState> = slots
                .iter()
                .enumerate()
                .map(|(j, slot)| match slot {
                    Some(comp) => component_state(comp.as_ref()),
                    None => pending_states[j].clone(),
                })
                .collect();
            self.report_progress(ctx, self.dependency_states(), progress);
        }
        self.components = slots
            .into_iter()
            .map(|slot| slot.expect("every component slot filled by its own future"))
            .collect();
        tracing::debug!(
            match_name = %self.name,
            elapsed = ?sw_comps.elapsed(),
            phase = "components_stop_end",
            "components stopped"
        );
        self.transition(ctx, ArenaLifecycleState::ComponentsStopped);

        self.active_playbooks.clear();

        self.transition(ctx, ArenaLifecycleState::DependenciesStopping);
        tracing::debug!(
            match_name = %self.name,
            dependency_count = self.dependencies.len(),
            phase = "dependencies_stop_begin",
            "stopping dependencies"
        );
        let sw_deps = Instant::now();
        let deps = std::mem::take(&mut self.dependencies);
        let dep_count = deps.len();
        let pending_states: Vec<DependencyState> =
            deps.iter().map(|d| dependency_state(d.as_ref())).collect();
        let mut in_flight: FuturesUnordered<_> = deps
            .into_iter()
            .enumerate()
            .map(|(i, mut dep)| async move {
                let fault = graceful_stop_dependency(&mut dep).await;
                (i, dep, fault)
            })
            .collect();
        let mut slots: Vec<Option<Dependency>> = (0..dep_count).map(|_| None).collect();
        while let Some((i, dep, fault)) = in_flight.next().await {
            if let Some(fault) = fault {
                faults.push(fault);
            }
            slots[i] = Some(dep);

            let progress: Vec<DependencyState> = slots
                .iter()
                .enumerate()
                .map(|(j, slot)| match slot {
                    Some(dep) => dependency_state(dep.as_ref()),
                    None => pending_states[j].clone(),
                })
                .collect();
            self.report_progress(ctx, progress, self.component_states());
        }
        self.dependencies = slots
            .into_iter()
            .map(|slot| slot.expect("every dependency slot filled by its own future"))
            .collect();
        tracing::debug!(
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
    fn attach_report_slot(&mut self, slot: MatchSlot) {
        self.slot = OnceLock::from(slot);
    }

    fn report_slot(&self) -> Option<MatchSlot> {
        self.slot.get().copied()
    }

    async fn start(&mut self, ctx: &ArenaLifecyclePublisher) -> Result<(), Vec<Fault>> {
        if self.started {
            return Ok(());
        }

        tracing::debug!(match_name = %self.name, phase = "start_begin", "starting");
        let sw = Instant::now();

        self.transition(ctx, ArenaLifecycleState::DependenciesStarting);
        let mut faults = self.start_dependencies(ctx).await;
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
        let mut faults = self.start_components(ctx).await;
        if !faults.is_empty() {
            faults.extend(self.graceful_teardown(ctx).await);
            return Err(faults);
        }
        self.transition(ctx, ArenaLifecycleState::ComponentsStarted);

        tracing::debug!(
            match_name = %self.name,
            elapsed = ?sw.elapsed(),
            phase = "start_end",
            "started"
        );
        self.started = true;
        Ok(())
    }

    async fn stop(&mut self, ctx: &ArenaLifecyclePublisher) -> Result<(), Vec<Fault>> {
        tracing::debug!(match_name = %self.name, phase = "stop_begin", "stopping");
        let sw = Instant::now();

        let faults = self.graceful_teardown(ctx).await;

        tracing::debug!(
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

    async fn force_stop_all(&mut self) -> Vec<Fault> {
        let mut faults = Vec::new();
        for comp in self.components.iter_mut().rev() {
            if let Some(fault) = force_stop_component(comp).await {
                faults.push(fault);
            }
        }
        for dep in self.dependencies.iter_mut().rev() {
            if let Some(fault) = force_stop_dependency(dep).await {
                faults.push(fault);
            }
        }
        faults
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

    fn attach_playbook_publisher(
        &self,
        arena_id: &str,
        publisher: Arc<dyn Publisher<PlaybookEvent>>,
    ) {
        for (playbook, exec_on_dependency_start) in &self.playbooks {
            let kind = if *exec_on_dependency_start {
                PlaybookKind::Startup
            } else {
                PlaybookKind::Scoped
            };
            playbook
                .lifecycle()
                .attach(arena_id, kind, Arc::clone(&publisher));
        }
    }

    async fn run_playbook(&self, identifier: &str) -> Option<Result<Box<dyn ActivePlaybook>, Fault>> {
        let pb = self
            .playbooks
            .iter()
            .find(|(p, _)| p.identifier() == identifier)?;
        let span = playbook_span(identifier);
        let outcome = AssertUnwindSafe(pb.0.run(&self.dependencies))
            .catch_unwind()
            .instrument(span)
            .await;
        match outcome {
            Ok(Ok(active)) => Some(Ok(active)),
            Ok(Err(fault)) => Some(Err(fault)),
            Err(payload) => {
                let fault = Fault::playbook(identifier, message::playbook_failed()).caused_by(
                    Fault::from_panic(identifier, Subject::Playbook, payload.as_ref()),
                );
                self.report_playbook_fault(identifier, fault.clone());
                Some(Err(fault))
            }
        }
    }
}
