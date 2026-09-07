use arena::dependency::RunnableDependency;
use arena_http::{
    a_response, get_requested_for, ok, post_requested_for, HeaderPattern, HttpDependency,
    HttpImpl,
};
use async_trait::async_trait;
use futures::FutureExt;

struct FakeHttpImpl {
    base_url: Option<String>,
}

#[async_trait]
impl HttpImpl for FakeHttpImpl {
    async fn start(&mut self, _port: u16, _image_name: &str, _image_tag: &str, _container_name: &str) -> Result<(), String> {
        self.base_url = Some("http://127.0.0.1:8080".to_string());
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.base_url = None;
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        true
    }
    fn release(&mut self) {}


    fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    fn admin_url(&self) -> Option<String> {
        self.base_url.as_deref().map(|url| format!("{url}/__admin"))
    }
}

struct OkReadinessCheck;

#[async_trait]
impl arena::healthcheck::ReadinessCheck for OkReadinessCheck {
    async fn is_ready(&self, _identifier: &str, _admin_url: &str, _timeout_ms: u64) -> Result<(), String> {
        Ok(())
    }
}

async fn started_http(identifier: &str) -> HttpDependency {
    let mut dep = HttpDependency::builder(identifier)
        .with_impl(FakeHttpImpl { base_url: None })
        .with_port(0)
        .with_readiness_check(OkReadinessCheck)
        .build();
    dep.start().await.expect("start should succeed");
    dep
}

#[test]
fn playbook_with_unstarted_dep_panics() {
    let dep = HttpDependency::builder("http-unstarted")
        .with_impl(FakeHttpImpl { base_url: None })
        .with_port(0)
        .with_readiness_check(OkReadinessCheck)
        .build();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = dep.playbook();
    }));

    assert!(outcome.is_err());
}

#[tokio::test]
async fn run_empty_playbook_sets_identifier() {
    let dep = started_http("http-empty").await;

    let active = dep.playbook().with_identifier("my-playbook").run().await;

    assert_eq!(
        arena::playbook::ActivePlaybook::identifier(&active),
        "my-playbook"
    );
    assert_eq!(active.admin_url(), "http://127.0.0.1:8080/__admin");
    active.persist();
}

#[tokio::test]
async fn run_default_identifier_includes_dependency_id() {
    let dep = started_http("http-default-id").await;

    let active = dep.playbook().run().await;

    assert!(arena::playbook::ActivePlaybook::identifier(&active).contains("http-default-id"));
    active.persist();
}

#[tokio::test]
async fn find_requests_unowned_criteria_panics() {
    let dep = started_http("http-unowned").await;
    let active = dep.playbook().with_identifier("unowned").run().await;

    let outcome = std::panic::AssertUnwindSafe(active.find_requests(get_requested_for("/nope")))
        .catch_unwind()
        .await;

    assert!(outcome.is_err());
    active.persist();
}

#[tokio::test]
async fn find_requests_header_criteria_panics() {
    let dep = started_http("http-header-crit").await;
    let active = dep.playbook().with_identifier("header-crit").run().await;

    let criteria =
        get_requested_for("/nope").with_header("X-Trace", HeaderPattern::equal_to("abc"));
    let outcome = std::panic::AssertUnwindSafe(active.find_requests(criteria))
        .catch_unwind()
        .await;

    assert!(outcome.is_err());
    active.persist();
}

#[tokio::test]
async fn verify_unowned_criteria_panics() {
    let dep = started_http("http-verify-unowned").await;
    let active = dep.playbook().with_identifier("verify-unowned").run().await;

    let outcome = std::panic::AssertUnwindSafe(active.verify(1, post_requested_for("/nope")))
        .catch_unwind()
        .await;

    assert!(outcome.is_err());
    active.persist();
}

#[tokio::test]
async fn verify_at_least_unowned_criteria_panics() {
    let dep = started_http("http-verify-al-unowned").await;
    let active = dep
        .playbook()
        .with_identifier("verify-al-unowned")
        .run()
        .await;

    let outcome =
        std::panic::AssertUnwindSafe(active.verify_at_least(1, post_requested_for("/nope")))
            .catch_unwind()
            .await;

    assert!(outcome.is_err());
    active.persist();
}

#[tokio::test]
async fn drop_active_playbook_no_mappings_no_panic() {
    let dep = started_http("http-drop-empty").await;
    let active = dep.playbook().with_identifier("drop-empty").run().await;
    drop(active);
}

#[tokio::test]
async fn playbook_verb_builders_chain_through_to_playbook() {
    let dep = started_http("http-builders").await;

    let get_playbook = dep
        .playbook()
        .get("/a")
        .with_header("X-Trace", HeaderPattern::equal_to("1"))
        .with_request_body(serde_json::json!({"k": "v"}))
        .with_request_body_containing("substr")
        .with_priority(5)
        .in_scenario("scenario-a")
        .when_state_is("Started")
        .will_set_state_to("next")
        .will_return(ok());
    let _ = get_playbook;

    let post_playbook = dep.playbook().post("/b").will_return_in_sequence(vec![ok(), a_response()]);
    let _ = post_playbook;

    let put_playbook = dep.playbook().put("/c").will_return(ok()).then_return(a_response());
    let _ = put_playbook;

    let delete_playbook = dep.playbook().delete("/d").will_return(ok());
    let _ = delete_playbook;
}

#[tokio::test]
async fn sequence_builder_expect_variants_build_playbook() {
    let dep = started_http("http-sequence").await;

    let exactly = dep
        .playbook()
        .get("/exactly")
        .will_return(ok())
        .expect_called(2)
        .into_playbook();
    let _ = exactly;

    let at_least = dep
        .playbook()
        .get("/at-least")
        .will_return(ok())
        .expect_called_at_least(1)
        .into_playbook();
    let _ = at_least;

    let never = dep
        .playbook()
        .get("/never")
        .will_return(ok())
        .expect_never_called()
        .into_playbook();
    let _ = never;
}

#[tokio::test]
async fn sequence_builder_verb_chains_produce_mapping_builders() {
    let dep = started_http("http-sequence-verbs").await;

    let get_next = dep
        .playbook()
        .get("/first")
        .will_return(ok())
        .get("/second")
        .will_return(ok());
    let _ = get_next;

    let post_next = dep
        .playbook()
        .get("/first")
        .will_return(ok())
        .post("/second")
        .will_return(ok());
    let _ = post_next;

    let put_next = dep
        .playbook()
        .get("/first")
        .will_return(ok())
        .put("/second")
        .will_return(ok());
    let _ = put_next;

    let delete_next = dep
        .playbook()
        .get("/first")
        .will_return(ok())
        .delete("/second")
        .will_return(ok());
    let _ = delete_next;
}

#[test]
#[should_panic(expected = "will_return_in_sequence requires at least one response")]
fn will_return_in_sequence_empty_responses_panics() {
    let dep = futures::executor::block_on(started_http("http-empty-sequence"));
    let _ = dep.playbook().get("/x").will_return_in_sequence(vec![]);
}
