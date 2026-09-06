use arena::dependency::{Dependency, RunnableDependency};
use arena::lifecycle::{Fault, RunnableState};
use arena_oauth::{OauthDependency, Provider, TokenError};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct ChildCalls {
    stopped: usize,
    released: usize,
    force_stopped: usize,
}

struct FakeChildDependency {
    calls: Arc<Mutex<ChildCalls>>,
    stop_fails: bool,
    state: RunnableState,
    faults: Vec<Fault>,
}

#[async_trait]
impl RunnableDependency for FakeChildDependency {
    fn identifier(&self) -> &str {
        "oauth-child"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn state(&self) -> RunnableState {
        self.state
    }

    fn faults(&self) -> &[Fault] {
        &self.faults
    }

    async fn start(&mut self) -> Result<(), Fault> {
        self.state = RunnableState::Started;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        self.calls.lock().unwrap().stopped += 1;
        if self.stop_fails {
            let fault = Fault::dependency(self.identifier(), "child stop failed");
            self.faults.push(fault.clone());
            self.state = RunnableState::Faulted;
            return Err(fault);
        }
        self.state = RunnableState::Stopped;
        Ok(())
    }

    async fn force_stop(&mut self) {
        self.calls.lock().unwrap().force_stopped += 1;
        self.state = RunnableState::Stopped;
    }

    fn release(&mut self) {
        self.calls.lock().unwrap().released += 1;
        self.state = RunnableState::Stopped;
    }

    fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}

    fn children(&self) -> &[Dependency] {
        &[]
    }

    fn children_mut(&mut self) -> &mut [Dependency] {
        &mut []
    }

    async fn soft_reset(&self) -> Result<(), Fault> {
        Ok(())
    }

    async fn hard_reset(&mut self) -> Result<(), Fault> {
        Ok(())
    }
}

fn setup_dependency(identifier: &str) -> OauthDependency {
    OauthDependency::builder(identifier).build()
}

fn setup_dependency_with_child(
    identifier: &str,
    stop_fails: bool,
) -> (OauthDependency, Arc<Mutex<ChildCalls>>) {
    let calls = Arc::new(Mutex::new(ChildCalls::default()));
    let mut dep = setup_dependency(identifier);
    dep.add_child(Box::new(FakeChildDependency {
        calls: calls.clone(),
        stop_fails,
        state: RunnableState::NotStarted,
        faults: Vec::new(),
    }));
    (dep, calls)
}

#[test]
fn base_url_before_start_returns_none() {
    assert_eq!(setup_dependency("oauth-base-url").base_url(), None);
}

#[test]
fn issuer_before_start_returns_none() {
    assert_eq!(setup_dependency("oauth-issuer").issuer(), None);
}

#[test]
fn issuer_for_provider_before_start_returns_none() {
    let dep = setup_dependency("oauth-issuer-for");

    assert_eq!(dep.issuer_for(&Provider::Custom { issuer_path: None }), None);
}

#[test]
fn verify_access_token_before_start_returns_not_running() {
    let dep = setup_dependency("oauth-verify");

    let error = dep
        .verify_access_token("not-a-token")
        .expect_err("verification should fail");

    assert!(matches!(error, TokenError::NotRunning));
}

#[tokio::test]
async fn stop_not_started_dependency_stops_children() {
    let (mut dep, calls) = setup_dependency_with_child("oauth-stop", false);

    dep.stop().await.expect("stop should succeed");

    assert_eq!(dep.state(), RunnableState::Stopped);
    assert_eq!(calls.lock().unwrap().stopped, 1);
}

#[tokio::test]
async fn stop_failing_child_returns_fault() {
    let (mut dep, _calls) = setup_dependency_with_child("oauth-stop-fault", true);

    let fault = dep.stop().await.expect_err("stop should fault");

    assert_eq!(fault.faults.len(), 1);
    assert_eq!(dep.state(), RunnableState::Faulted);
    assert_eq!(dep.faults().len(), 1);
}

#[test]
fn release_dependency_releases_children() {
    let (mut dep, calls) = setup_dependency_with_child("oauth-release", false);

    dep.release();

    assert_eq!(dep.state(), RunnableState::Stopped);
    assert_eq!(calls.lock().unwrap().released, 1);
}

#[tokio::test]
async fn force_stop_dependency_force_stops_children() {
    let (mut dep, calls) = setup_dependency_with_child("oauth-force-stop", false);

    dep.force_stop().await;

    assert_eq!(dep.state(), RunnableState::Stopped);
    assert_eq!(calls.lock().unwrap().force_stopped, 1);
}

#[tokio::test]
async fn hard_reset_not_started_dependency_returns_ok() {
    let mut dep = setup_dependency("oauth-hard-reset");

    assert!(dep.hard_reset().await.is_ok());
}
