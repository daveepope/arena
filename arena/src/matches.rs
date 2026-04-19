use super::component::Component;
use super::dependency::Dependency;
use super::dependency::RunnableDependency;
use super::playbook::{ActivePlaybook, Playbook};
use async_trait::async_trait;
use futures::future::join_all;
use std::time::Instant;

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
}

#[async_trait]
impl MatchTrait for Match {
    async fn start(&mut self) {
        if self.started {
            return;
        }

        log::info!("[Match-{}] starting.", self.name);
        let sw = Instant::now();

        let deps = std::mem::take(&mut self.dependencies);

        let mut started = join_all(deps.into_iter().enumerate().map(|(i, mut dep)| async move {
            dep.start().await;
            (i, dep)
        }))
        .await;

        started.sort_by_key(|(i, _)| *i);
        self.dependencies = started.into_iter().map(|(_, dep)| dep).collect();

        let startup: Vec<&dyn Playbook> = self
            .playbooks
            .iter()
            .filter_map(|(pb, exec_on_start)| exec_on_start.then(|| pb.as_ref()))
            .collect();

        if !startup.is_empty() {
            log::info!(
                "[Match-{}] running {} playbook(s) in parallel.",
                self.name,
                startup.len()
            );
            let sw_pb = Instant::now();
            let deps_ref: &[Dependency] = &self.dependencies;
            let actives = join_all(
                startup
                    .iter()
                    .map(|pb| async move { pb.run(deps_ref).await }),
            )
            .await;
            self.active_playbooks.extend(actives);
            log::debug!(
                "[Match-{}] playbooks completed in {:?}.",
                self.name,
                sw_pb.elapsed()
            );
        }

        let comps = std::mem::take(&mut self.components);

        let mut started_comps = join_all(comps.into_iter().enumerate().map(|(i, mut comp)| async move {
            comp.start().await;
            (i, comp)
        }))
        .await;

        started_comps.sort_by_key(|(i, _)| *i);
        self.components = started_comps.into_iter().map(|(_, comp)| comp).collect();

        log::debug!(
            "[Match-{}] start complete in {:?}.",
            self.name,
            sw.elapsed()
        );
        log::info!("[Match-{}] started.", self.name);
        self.started = true;
    }

    async fn stop(&mut self) {
        if !self.started {
            return;
        }

        log::info!("[Match-{}] stopping.", self.name);
        let sw = Instant::now();

        let comps = std::mem::take(&mut self.components);

        let mut stopped_comps = join_all(comps.into_iter().enumerate().map(|(i, mut comp)| async move {
            comp.stop().await;
            (i, comp)
        }))
        .await;

        stopped_comps.sort_by_key(|(i, _)| *i);
        self.components = stopped_comps.into_iter().map(|(_, comp)| comp).collect();

        self.active_playbooks.clear();

        let deps = std::mem::take(&mut self.dependencies);

        let mut stopped = join_all(deps.into_iter().enumerate().map(|(i, mut dep)| async move {
            dep.stop().await;
            (i, dep)
        }))
        .await;

        stopped.sort_by_key(|(i, _)| *i);
        self.dependencies = stopped.into_iter().map(|(_, dep)| dep).collect();

        log::debug!(
            "[Match-{}] stop complete in {:?}.",
            self.name,
            sw.elapsed()
        );
        log::info!("[Match-{}] stopped.", self.name);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency::RunnableDependency;
    use async_trait::async_trait;
    use futures::future::{select, Either};
    use std::any::Any;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::sync::Barrier;

    struct StubDependency {
        identifier: String,
        started: Arc<Mutex<bool>>,
    }

    #[async_trait]
    impl RunnableDependency for StubDependency {
        fn identifier(&self) -> &str {
            &self.identifier
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
        async fn start(&mut self) {
            *self.started.lock().unwrap() = true;
        }
        async fn stop(&mut self) {
            *self.started.lock().unwrap() = false;
        }
        fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}
        async fn soft_reset(&self) {}
        async fn hard_reset(&mut self) {}
    }

    struct RecordingPlaybook {
        identifier: String,
        run_log: Arc<Mutex<Vec<String>>>,
        drop_log: Arc<Mutex<Vec<String>>>,
        dep_started_snapshot: Arc<Mutex<Option<bool>>>,
        dep_to_check: String,
        rendezvous: Arc<Barrier>,
    }

    #[async_trait]
    impl Playbook for RecordingPlaybook {
        fn identifier(&self) -> &str {
            &self.identifier
        }

        async fn run(&self, dependencies: &[Dependency]) -> Box<dyn ActivePlaybook> {
            let snap = dependencies
                .iter()
                .find(|d| d.identifier() == self.dep_to_check)
                .and_then(|d| d.as_any().downcast_ref::<StubDependency>())
                .map(|s| *s.started.lock().unwrap());
            *self.dep_started_snapshot.lock().unwrap() = snap;

            self.rendezvous.wait().await;

            self.run_log.lock().unwrap().push(self.identifier.clone());

            Box::new(RecordingActive {
                identifier: self.identifier.clone(),
                drop_log: self.drop_log.clone(),
            })
        }
    }

    struct RecordingActive {
        identifier: String,
        drop_log: Arc<Mutex<Vec<String>>>,
    }

    impl ActivePlaybook for RecordingActive {
        fn identifier(&self) -> &str {
            &self.identifier
        }
    }

    impl Drop for RecordingActive {
        fn drop(&mut self) {
            self.drop_log.lock().unwrap().push(self.identifier.clone());
        }
    }

    async fn within<F: std::future::Future>(budget: Duration, what: &str, fut: F) -> F::Output {
        let fut = std::pin::pin!(fut);
        let deadline = futures_timer::Delay::new(budget);
        match select(fut, deadline).await {
            Either::Left((out, _)) => out,
            Either::Right(_) => panic!("{what} did not complete within {budget:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn register_playbook_runs_after_dependencies_started_and_in_parallel() {
        let _ = env_logger::builder().is_test(true).try_init();

        let dep_started = Arc::new(Mutex::new(false));
        let dep = StubDependency {
            identifier: "dep-1".to_string(),
            started: dep_started.clone(),
        };

        let run_log = Arc::new(Mutex::new(Vec::<String>::new()));
        let drop_log = Arc::new(Mutex::new(Vec::<String>::new()));
        let snap_1 = Arc::new(Mutex::new(None));
        let snap_2 = Arc::new(Mutex::new(None));
        let rendezvous = Arc::new(Barrier::new(2));

        let pb_1 = Box::new(RecordingPlaybook {
            identifier: "pb-1".to_string(),
            run_log: run_log.clone(),
            drop_log: drop_log.clone(),
            dep_started_snapshot: snap_1.clone(),
            dep_to_check: "dep-1".to_string(),
            rendezvous: rendezvous.clone(),
        });
        let pb_2 = Box::new(RecordingPlaybook {
            identifier: "pb-2".to_string(),
            run_log: run_log.clone(),
            drop_log: drop_log.clone(),
            dep_started_snapshot: snap_2.clone(),
            dep_to_check: "dep-1".to_string(),
            rendezvous: rendezvous.clone(),
        });

        let mut a_match = Match::new("test-match", vec![Box::new(dep)], vec![])
            .register_playbook(pb_1, true)
            .register_playbook(pb_2, true);

        within(Duration::from_millis(50), "Match::start", a_match.start()).await;

        assert_eq!(*snap_1.lock().unwrap(), Some(true));
        assert_eq!(*snap_2.lock().unwrap(), Some(true));

        let ran = run_log.lock().unwrap().clone();
        assert_eq!(ran.len(), 2);
        assert!(ran.contains(&"pb-1".to_string()));
        assert!(ran.contains(&"pb-2".to_string()));

        assert!(drop_log.lock().unwrap().is_empty());

        a_match.stop().await;

        let dropped = drop_log.lock().unwrap().clone();
        assert_eq!(dropped.len(), 2);
        assert!(dropped.contains(&"pb-1".to_string()));
        assert!(dropped.contains(&"pb-2".to_string()));
    }

    struct PanicOnRunPlaybook {
        identifier: String,
    }

    #[async_trait]
    impl Playbook for PanicOnRunPlaybook {
        fn identifier(&self) -> &str {
            &self.identifier
        }

        async fn run(&self, _: &[Dependency]) -> Box<dyn ActivePlaybook> {
            panic!("playbook '{}' should not have run", self.identifier);
        }
    }

    #[tokio::test]
    async fn register_playbook_skips_execution_when_exec_on_start_is_false() {
        let _ = env_logger::builder().is_test(true).try_init();

        let dep = StubDependency {
            identifier: "dep-1".to_string(),
            started: Arc::new(Mutex::new(false)),
        };

        let pb = PanicOnRunPlaybook {
            identifier: "pb-skip".to_string(),
        };

        let mut a_match = Match::new("test-match", vec![Box::new(dep)], vec![])
            .register_playbook(Box::new(pb), false);

        a_match.start().await;
        a_match.stop().await;
    }
}
