use arena::dependency::RunnableDependency;
use arena::matches::Match;
use arena::playbook::{ActivePlaybook, Playbook};
use arena::{ClosedArena, Dependency, MatchTrait};
use async_trait::async_trait;
use futures::FutureExt;
use mockall::mock;
use std::any::Any;

mock! {
    Match {}
    #[async_trait]
    impl MatchTrait for Match {
        async fn start(&mut self);
        async fn stop(&mut self);
    }
}

struct StubDependency {
    identifier: String,
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
    async fn start(&mut self) {}
    async fn stop(&mut self) {}
    fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}
    fn children(&self) -> &[Dependency] {
        &[]
    }
    fn children_mut(&mut self) -> &mut [Dependency] {
        &mut []
    }
    async fn soft_reset(&self) {}
    async fn hard_reset(&mut self) {}
}

struct StubActivePlaybook {
    identifier: String,
}

impl ActivePlaybook for StubActivePlaybook {
    fn identifier(&self) -> &str {
        &self.identifier
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct StubPlaybook {
    identifier: String,
}

#[async_trait]
impl Playbook for StubPlaybook {
    fn identifier(&self) -> &str {
        &self.identifier
    }
    async fn run(&self, _dependencies: &[Dependency]) -> Box<dyn ActivePlaybook> {
        Box::new(StubActivePlaybook {
            identifier: self.identifier.clone(),
        })
    }
}

#[tokio::test]
async fn open_happy_path_starts_and_stops_matches() {
    let matches: Vec<Box<dyn MatchTrait>> = vec![
        Box::new(create_and_setup_stub_match()),
        Box::new(create_and_setup_stub_match()),
    ];

    let closed = ClosedArena::new("TestArena".to_string(), matches);
    let open = closed.open().await;
    let _closed = open.close().await;
}

#[test]
fn open_happy_path_drop_closes_open_arena() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let matches: Vec<Box<dyn MatchTrait>> = vec![Box::new(create_and_setup_stub_match())];

        let closed = ClosedArena::new("TestArena".to_string(), matches);
        let open = closed.open().await;

        drop(open);
    });
}

#[tokio::test]
async fn open_later_match_panic_stops_earlier_match() {
    let mut match_ok = MockMatch::new();
    match_ok.expect_start().times(1).returning(|| ());
    match_ok.expect_stop().times(1).returning(|| ());

    let mut match_fail = MockMatch::new();
    match_fail
        .expect_start()
        .times(1)
        .returning(|| panic!("match start failed"));

    let closed = ClosedArena::new(
        "TestArena".to_string(),
        vec![Box::new(match_ok), Box::new(match_fail)],
    );

    let outcome = std::panic::AssertUnwindSafe(closed.open())
        .catch_unwind()
        .await;
    assert!(outcome.is_err());
}

fn create_and_setup_stub_match() -> MockMatch {
    let mut stub_match = MockMatch::new();
    stub_match.expect_start().times(1).returning(|| ());
    stub_match.expect_stop().times(1).returning(|| ());
    stub_match
}

#[tokio::test]
async fn dependency_found_in_later_match_returns_dependency() {
    let empty_match = create_and_setup_stub_match();

    let real_match = Match::new(
        "real-match",
        vec![Box::new(StubDependency {
            identifier: "dep-1".to_string(),
        })],
        vec![],
    )
    .register_playbook(
        Box::new(StubPlaybook {
            identifier: "pb-1".to_string(),
        }),
        false,
    );

    let matches: Vec<Box<dyn MatchTrait>> =
        vec![Box::new(empty_match), Box::new(real_match)];
    let closed = ClosedArena::new("TestArena".to_string(), matches);
    let mut open = closed.open().await;

    let found = open.dependency("dep-1");
    assert!(found.is_some());
    assert_eq!(found.unwrap().identifier(), "dep-1");
    assert!(open.dependency("missing").is_none());

    let found_mut = open.dependency_mut("dep-1");
    assert!(found_mut.is_some());
    assert_eq!(found_mut.unwrap().identifier(), "dep-1");

    let active = open.run_playbook("pb-1").await;
    assert!(active.is_some());
    assert_eq!(active.unwrap().identifier(), "pb-1");
    assert!(open.run_playbook("missing").await.is_none());

    let _closed = open.close().await;
}
