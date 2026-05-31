use arena::dependency::RunnableDependency;
use arena::matches::{Match, MatchTrait};
use arena::playbook::{ActivePlaybook, Playbook};
use arena::Dependency;
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

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for RecordingActive {
    fn drop(&mut self) {
        self.drop_log.lock().unwrap().push(self.identifier.clone());
    }
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

async fn within<F: std::future::Future>(budget: Duration, what: &str, fut: F) -> F::Output {
    let fut = std::pin::pin!(fut);
    let deadline = futures_timer::Delay::new(budget);
    match select(fut, deadline).await {
        Either::Left((out, _)) => out,
        Either::Right(_) => panic!("{what} did not complete within {budget:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn register_playbook_exec_on_start_runs_after_deps_in_parallel() {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();

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

#[tokio::test]
async fn register_playbook_exec_on_start_false_skips_run() {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();

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

#[tokio::test]
async fn run_playbook_known_id_returns_active() {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();

    let dep_started = Arc::new(Mutex::new(false));
    let dep = StubDependency {
        identifier: "dep-1".to_string(),
        started: dep_started.clone(),
    };

    let run_log = Arc::new(Mutex::new(Vec::<String>::new()));
    let drop_log = Arc::new(Mutex::new(Vec::<String>::new()));
    let snap = Arc::new(Mutex::new(None));
    let rendezvous = Arc::new(Barrier::new(1));

    let pb = Box::new(RecordingPlaybook {
        identifier: "lookup-pb".to_string(),
        run_log: run_log.clone(),
        drop_log: drop_log.clone(),
        dep_started_snapshot: snap.clone(),
        dep_to_check: "dep-1".to_string(),
        rendezvous: rendezvous.clone(),
    });

    let mut a_match = Match::new("test-match", vec![Box::new(dep)], vec![])
        .register_playbook(pb, false);

    a_match.start().await;

    let active = a_match.run_playbook("lookup-pb").await;
    assert!(active.is_some());
    assert_eq!(run_log.lock().unwrap().clone(), vec!["lookup-pb".to_string()]);

    drop(active);
    assert_eq!(drop_log.lock().unwrap().clone(), vec!["lookup-pb".to_string()]);

    a_match.stop().await;
}

#[tokio::test]
async fn run_playbook_unknown_id_returns_none() {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();

    let dep = StubDependency {
        identifier: "dep-1".to_string(),
        started: Arc::new(Mutex::new(false)),
    };

    let mut a_match = Match::new("test-match", vec![Box::new(dep)], vec![]);
    a_match.start().await;

    let active = a_match.run_playbook("does-not-exist").await;
    assert!(active.is_none());

    a_match.stop().await;
}
