use super::component::Component;
use super::dependency::Dependency;
use super::dependency::RunnableDependency;
use super::playbook::{ActivePlaybook, Playbook};
use async_trait::async_trait;
use futures::future::join_all;
use futures::FutureExt;
use std::panic::{AssertUnwindSafe, resume_unwind};
use std::time::Instant;

async fn stop_dependencies(deps: &mut [Dependency]) {
    for dep in deps.iter_mut().rev() {
        dep.stop().await;
    }
}

async fn stop_components(comps: &mut [Component]) {
    for comp in comps.iter_mut().rev() {
        comp.stop().await;
    }
}

#[async_trait]
pub trait MatchTrait: Send + Sync {
    async fn start(&mut self);
    async fn stop(&mut self);

    fn dependency(&self, _identifier: &str) -> Option<&(dyn RunnableDependency + '_)> {
        None
    }

    fn dependency_mut(&mut self, _identifier: &str) -> Option<&mut (dyn RunnableDependency + '_)> {
        None
    }

    async fn run_playbook(&self, _identifier: &str) -> Option<Box<dyn ActivePlaybook>> {
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

    async fn rollback_after_failed_start(&mut self) {
        stop_components(&mut self.components).await;
        self.active_playbooks.clear();
        stop_dependencies(&mut self.dependencies).await;
    }
}

#[async_trait]
impl MatchTrait for Match {
    async fn start(&mut self) {
        if self.started {
            return;
        }

        tracing::info!(match_name = %self.name, phase = "start_begin", "starting");
        let sw = Instant::now();

        let dep_count = self.dependencies.len();
        let deps = std::mem::take(&mut self.dependencies);
        let match_label = self.name.clone();

        if dep_count > 0 {
            tracing::info!(
                match_name = %self.name,
                dependency_count = dep_count,
                phase = "dependencies_start_begin",
                "starting dependencies"
            );
        }
        let sw_deps_batch = Instant::now();
        let outcomes = join_all(deps.into_iter().enumerate().map(|(i, mut dep)| {
            let match_label = match_label.clone();
            async move {
                let id = dep.identifier().to_string();
                let sw_one = Instant::now();
                let outcome = AssertUnwindSafe(async {
                    dep.start().await;
                    dep
                })
                .catch_unwind()
                .await;
                (i, id, match_label, sw_one, outcome)
            }
        }))
        .await;

        let mut dep_panics = Vec::new();
        let mut started = Vec::with_capacity(dep_count);
        for (i, id, match_label, sw_one, outcome) in outcomes {
            match outcome {
                Ok(dep) => {
                    tracing::info!(
                        match_name = %match_label,
                        dependency = %id,
                        elapsed = ?sw_one.elapsed(),
                        phase = "dependency_start_complete",
                        "dependency started"
                    );
                    started.push((i, dep));
                }
                Err(payload) => dep_panics.push(payload),
            }
        }

        if !dep_panics.is_empty() {
            started.sort_by_key(|(i, _)| *i);
            self.dependencies = started.into_iter().map(|(_, dep)| dep).collect();
            self.rollback_after_failed_start().await;
            resume_unwind(dep_panics.into_iter().next().unwrap());
        }

        started.sort_by_key(|(i, _)| *i);
        self.dependencies = started.into_iter().map(|(_, dep)| dep).collect();

        if dep_count > 0 {
            tracing::info!(
                match_name = %self.name,
                elapsed = ?sw_deps_batch.elapsed(),
                dependency_count = dep_count,
                phase = "dependencies_start_end",
                "dependencies started"
            );
        }

        let startup: Vec<&dyn Playbook> = self
            .playbooks
            .iter()
            .filter_map(|(pb, exec_on_start)| exec_on_start.then(|| pb.as_ref()))
            .collect();

        if !startup.is_empty() {
            tracing::info!(
                match_name = %self.name,
                playbook_count = startup.len(),
                phase = "playbook_parallel_begin",
                "running playbooks in parallel"
            );
            let sw_pb = Instant::now();
            let deps_ref: &[Dependency] = &self.dependencies;
            let match_label_for_pb = self.name.clone();
            let outcomes = join_all((0..startup.len()).map(|idx| {
                let pb = startup[idx];
                let id = pb.identifier().to_string();
                let match_label_for_pb = match_label_for_pb.clone();
                async move {
                    let sw_one = Instant::now();
                    let outcome = AssertUnwindSafe(async {
                        pb.run(deps_ref).await
                    })
                    .catch_unwind()
                    .await;
                    (id, match_label_for_pb, sw_one, outcome)
                }
            }))
            .await;

            let mut pb_panics = Vec::new();
            let mut actives = Vec::with_capacity(outcomes.len());
            for (id, match_label_for_pb, sw_one, outcome) in outcomes {
                match outcome {
                    Ok(active) => {
                        tracing::info!(
                            match_name = %match_label_for_pb,
                            playbook = %id,
                            elapsed = ?sw_one.elapsed(),
                            phase = "playbook_run_complete",
                            "playbook applied"
                        );
                        actives.push(active);
                    }
                    Err(payload) => pb_panics.push(payload),
                }
            }

            if !pb_panics.is_empty() {
                self.active_playbooks = actives;
                self.rollback_after_failed_start().await;
                resume_unwind(pb_panics.into_iter().next().unwrap());
            }

            self.active_playbooks = actives;
            tracing::info!(
                match_name = %self.name,
                elapsed = ?sw_pb.elapsed(),
                phase = "playbook_parallel_end",
                "playbooks complete"
            );
        }

        let comp_count = self.components.len();
        if comp_count > 0 {
            tracing::info!(
                match_name = %self.name,
                component_count = comp_count,
                phase = "components_start_begin",
                "starting components"
            );
        }
        let sw_comps_batch = Instant::now();
        let comps = std::mem::take(&mut self.components);
        let outcomes = join_all(comps.into_iter().enumerate().map(|(i, mut comp)| {
            let match_label = match_label.clone();
            async move {
                let sw_one = Instant::now();
                let outcome = AssertUnwindSafe(async {
                    comp.start().await;
                    comp
                })
                .catch_unwind()
                .await;
                (i, match_label, sw_one, outcome)
            }
        }))
        .await;

        let mut comp_panics = Vec::new();
        let mut started_comps = Vec::with_capacity(comp_count);
        for (i, match_label, sw_one, outcome) in outcomes {
            match outcome {
                Ok(comp) => {
                    tracing::info!(
                        match_name = %match_label,
                        component_index = i,
                        elapsed = ?sw_one.elapsed(),
                        phase = "component_start_complete",
                        "component started"
                    );
                    started_comps.push((i, comp));
                }
                Err(payload) => comp_panics.push(payload),
            }
        }

        if !comp_panics.is_empty() {
            started_comps.sort_by_key(|(i, _)| *i);
            self.components = started_comps.into_iter().map(|(_, comp)| comp).collect();
            self.rollback_after_failed_start().await;
            resume_unwind(comp_panics.into_iter().next().unwrap());
        }

        started_comps.sort_by_key(|(i, _)| *i);
        self.components = started_comps.into_iter().map(|(_, comp)| comp).collect();

        if comp_count > 0 {
            tracing::info!(
                match_name = %self.name,
                elapsed = ?sw_comps_batch.elapsed(),
                component_count = comp_count,
                phase = "components_start_end",
                "components started"
            );
        }

        tracing::info!(
            match_name = %self.name,
            elapsed = ?sw.elapsed(),
            phase = "start_end",
            "started"
        );
        self.started = true;
    }

    async fn stop(&mut self) {
        if !self.started {
            self.rollback_after_failed_start().await;
            return;
        }

        tracing::info!(match_name = %self.name, phase = "stop_begin", "stopping");
        let sw = Instant::now();

        let comp_count = self.components.len();
        let match_label = self.name.clone();
        if comp_count > 0 {
            tracing::info!(
                match_name = %self.name,
                component_count = comp_count,
                phase = "components_stop_begin",
                "stopping components"
            );
        }
        let sw_comps_batch = Instant::now();
        let comps = std::mem::take(&mut self.components);

        let mut stopped_comps = join_all(comps.into_iter().enumerate().map(|(i, mut comp)| {
            let match_label = match_label.clone();
            async move {
                let sw_one = Instant::now();
                comp.stop().await;
                tracing::info!(
                    match_name = %match_label,
                    component_index = i,
                    elapsed = ?sw_one.elapsed(),
                    phase = "component_stop_complete",
                    "component stopped"
                );
                (i, comp)
            }
        }))
        .await;

        stopped_comps.sort_by_key(|(i, _)| *i);
        self.components = stopped_comps.into_iter().map(|(_, comp)| comp).collect();

        if comp_count > 0 {
            tracing::info!(
                match_name = %self.name,
                elapsed = ?sw_comps_batch.elapsed(),
                component_count = comp_count,
                phase = "components_stop_end",
                "components stopped"
            );
        }

        self.active_playbooks.clear();

        let dep_count = self.dependencies.len();
        let deps = std::mem::take(&mut self.dependencies);

        if dep_count > 0 {
            tracing::info!(
                match_name = %self.name,
                dependency_count = dep_count,
                phase = "dependencies_stop_begin",
                "stopping dependencies"
            );
        }
        let sw_deps_batch = Instant::now();

        let mut stopped = join_all(deps.into_iter().enumerate().map(|(i, mut dep)| {
            let match_label = match_label.clone();
            async move {
                let id = dep.identifier().to_string();
                let sw_one = Instant::now();
                dep.stop().await;
                tracing::info!(
                    match_name = %match_label,
                    dependency = %id,
                    elapsed = ?sw_one.elapsed(),
                    phase = "dependency_stop_complete",
                    "dependency stopped"
                );
                (i, dep)
            }
        }))
        .await;

        stopped.sort_by_key(|(i, _)| *i);
        self.dependencies = stopped.into_iter().map(|(_, dep)| dep).collect();

        if dep_count > 0 {
            tracing::info!(
                match_name = %self.name,
                elapsed = ?sw_deps_batch.elapsed(),
                dependency_count = dep_count,
                phase = "dependencies_stop_end",
                "dependencies stopped"
            );
        }

        tracing::info!(
            match_name = %self.name,
            elapsed = ?sw.elapsed(),
            phase = "stop_end",
            "stopped"
        );
        self.started = false;
    }

    fn dependency(&self, identifier: &str) -> Option<&(dyn RunnableDependency + '_)> {
        self.dependencies
            .iter()
            .map(|d| d.as_ref())
            .find(|d| d.identifier() == identifier)
    }

    fn dependency_mut(&mut self, identifier: &str) -> Option<&mut (dyn RunnableDependency + '_)> {
        for dep in &mut self.dependencies {
            if dep.identifier() == identifier {
                return Some(dep.as_mut());
            }
        }
        None
    }

    async fn run_playbook(&self, identifier: &str) -> Option<Box<dyn ActivePlaybook>> {
        let pb = self
            .playbooks
            .iter()
            .find(|(p, _)| p.identifier() == identifier)?;
        Some(pb.0.run(&self.dependencies).await)
    }
}
