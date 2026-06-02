use arena::{ClosedArena, MatchTrait};
use async_trait::async_trait;
use futures::FutureExt;
use mockall::mock;

mock! {
    Match {}
    #[async_trait]
    impl MatchTrait for Match {
        async fn start(&mut self);
        async fn stop(&mut self);
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
