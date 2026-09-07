use arena::lifecycle::{Fault, RunnableState};
use arena::dependency::{Dependency, RunnableDependency};
use arena_oauth::OauthDependency;
use futures::FutureExt;

#[test]
fn drop_unstarted_dep_does_not_panic() {
    let dep = OauthDependency::builder("oauth-drop").build();
    drop(dep);
}

#[tokio::test]
async fn stop_then_drop_does_not_panic() {
    let mut dep = OauthDependency::builder("oauth-drop").build();
    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");
    drop(dep);
}

#[test]
fn drop_running_dep_stops_server() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut dep = OauthDependency::builder("oauth-drop").build();
        dep.start().await.expect("start should succeed");
        assert!(dep.base_url().is_some());
        drop(dep);
    });
}

struct PanickingOauthChild;

#[async_trait::async_trait]
impl RunnableDependency for PanickingOauthChild {
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
        RunnableState::NotStarted
    }

    fn faults(&self) -> &[Fault] {
        &[]
    }

    async fn force_stop(&mut self) {}
    fn release(&mut self) {}

    async fn start(&mut self) -> Result<(), Fault> {
        panic!("child dependency start failed");
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        Ok(())
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

#[tokio::test]
async fn start_child_panic_then_drop_does_not_panic() {
    let mut dep = OauthDependency::builder("oauth-drop")
        .with_child_dependencies(vec![Box::new(PanickingOauthChild)])
        .build();

    let start_outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await.expect("start should succeed");
    })
    .catch_unwind()
    .await;
    assert!(start_outcome.is_err());
    drop(dep);
}
