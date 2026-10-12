use std::sync::{Arc, Mutex};
use std::time::Duration;

use arena::component::{Component, RunnableComponent};
use arena::dependency::{Dependency, RunnableDependency};
use arena::lifecycle::{ArenaLifecycleObserver, ArenaLifecycleState, ArenaState, Fault};
use arena::{ClosedArena, Match, MatchTrait, RunnableState};
use arena_dashboard::{app, AppState, HttpForwardingObserver};
use axum::extract::State;
use axum::routing::post;
use axum::Router;
use futures::StreamExt;

#[derive(Clone, Default)]
struct CaptureState {
    bodies: Arc<Mutex<Vec<String>>>,
}

async fn capture_ingest(State(state): State<CaptureState>, body: String) -> &'static str {
    state.bodies.lock().unwrap().push(body);
    "ok"
}

async fn spawn_capture_server() -> (String, Arc<Mutex<Vec<String>>>) {
    let state = CaptureState::default();
    let bodies = Arc::clone(&state.bodies);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let router = Router::new()
        .route("/ingest", post(capture_ingest))
        .with_state(state);
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    (format!("http://{addr}"), bodies)
}

struct InstantDependency {
    identifier: String,
    state: RunnableState,
    children: Vec<Dependency>,
}

impl InstantDependency {
    fn new(identifier: &str) -> Self {
        Self {
            identifier: identifier.to_string(),
            state: RunnableState::NotStarted,
            children: Vec::new(),
        }
    }

    fn with_child(mut self, child: Dependency) -> Self {
        self.children.push(child);
        self
    }

    fn into_dependency(self) -> Dependency {
        Box::new(self)
    }
}

#[async_trait::async_trait]
impl RunnableDependency for InstantDependency {
    fn identifier(&self) -> &str {
        &self.identifier
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
        &[]
    }

    async fn start(&mut self) -> Result<(), Fault> {
        for child in self.children.iter_mut() {
            arena::dependency::start_child(child).await?;
        }
        self.state = RunnableState::Started;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        self.state = RunnableState::Stopped;
        for child in self.children.iter_mut() {
            arena::dependency::stop_child(child).await?;
        }
        Ok(())
    }

    async fn force_stop(&mut self) {
        for child in self.children.iter_mut() {
            arena::dependency::force_stop_child(child).await;
        }
        self.state = RunnableState::Stopped;
    }

    fn release(&mut self) {
        for child in self.children.iter_mut() {
            arena::dependency::release_child(child);
        }
        self.state = RunnableState::Stopped;
    }

    fn add_child(&mut self, dep: Dependency) {
        self.children.push(dep);
    }

    fn children(&self) -> &[Dependency] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Dependency] {
        &mut self.children
    }

    async fn soft_reset(&self) -> Result<(), Fault> {
        Ok(())
    }

    async fn hard_reset(&mut self) -> Result<(), Fault> {
        Ok(())
    }
}

struct InstantComponent {
    identifier: String,
    state: RunnableState,
    children: Vec<Component>,
}

impl InstantComponent {
    fn new(identifier: &str) -> Self {
        Self {
            identifier: identifier.to_string(),
            state: RunnableState::NotStarted,
            children: Vec::new(),
        }
    }

    fn with_child(mut self, child: Component) -> Self {
        self.children.push(child);
        self
    }

    fn into_component(self) -> Component {
        Box::new(self)
    }
}

#[async_trait::async_trait]
impl RunnableComponent for InstantComponent {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn state(&self) -> RunnableState {
        self.state
    }

    fn faults(&self) -> &[Fault] {
        &[]
    }

    async fn start(&mut self) -> Result<(), Fault> {
        for child in self.children.iter_mut() {
            arena::component::start_child(child).await?;
        }
        self.state = RunnableState::Started;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        self.state = RunnableState::Stopped;
        for child in self.children.iter_mut() {
            arena::component::stop_child(child).await?;
        }
        Ok(())
    }

    async fn force_stop(&mut self) {
        for child in self.children.iter_mut() {
            arena::component::force_stop_child(child).await;
        }
        self.state = RunnableState::Stopped;
    }

    fn release(&mut self) {
        for child in self.children.iter_mut() {
            arena::component::release_child(child);
        }
        self.state = RunnableState::Stopped;
    }

    fn add_child(&mut self, child: Component) {
        self.children.push(child);
    }

    fn children(&self) -> &[Component] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Component] {
        &mut self.children
    }
}

async fn spawn_real_dashboard_server() -> String {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app(AppState::new())).await;
    });
    format!("http://{addr}")
}

#[tokio::test(flavor = "multi_thread")]
async fn closed_arena_open_and_close_forwards_real_nested_state_to_dashboard() {
    let base_url = spawn_real_dashboard_server().await;

    let dependency = InstantDependency::new("dep-a")
        .with_child(InstantDependency::new("dep-a-child").into_dependency());
    let component = InstantComponent::new("comp-a")
        .with_child(InstantComponent::new("comp-a-child").into_component());
    let matches: Vec<Box<dyn MatchTrait>> = vec![Box::new(Match::new(
        "match-1",
        vec![dependency.into_dependency()],
        vec![component.into_component()],
    ))];
    let closed = ClosedArena::new("real-arena".to_string(), matches)
        .observe(Arc::new(HttpForwardingObserver::new(base_url.clone())));

    let open = closed.open().await.expect("real arena should open");
    open.close().await.expect("real arena should close");

    // the forwarding observer ships each state over an internal channel to
    // its own background thread; give it a moment to deliver before reading
    // the dashboard's replay.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{base_url}/events"))
        .send()
        .await
        .expect("connect to events");
    let mut stream = response.bytes_stream();
    let chunk = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("sse event within timeout")
        .expect("sse stream item")
        .expect("sse chunk bytes");
    let text = String::from_utf8_lossy(&chunk);

    assert!(text.contains(r#""id":"real-arena""#), "{text}");
    assert!(text.contains(r#""state":"arena_closed""#), "{text}");
    assert!(text.contains(r#""id":"dep-a""#), "{text}");
    assert!(text.contains(r#""id":"dep-a-child""#), "{text}");
    assert!(text.contains(r#""id":"comp-a""#), "{text}");
    assert!(text.contains(r#""id":"comp-a-child""#), "{text}");
    assert!(
        !text.contains("dependency_ids"),
        "a real arena never emits dependency_ids: {text}"
    );
}

#[tokio::test]
async fn http_forwarding_observer_on_state_forwards_via_http() {
    let (base_url, bodies) = spawn_capture_server().await;

    let observer = HttpForwardingObserver::new(base_url);
    let state = ArenaState::new(
        "http-forward-arena",
        ArenaLifecycleState::ArenaOpen,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    observer.on_state(&state);

    tokio::task::spawn_blocking(move || drop(observer))
        .await
        .expect("drop observer");

    let documents = bodies.lock().unwrap();
    assert_eq!(documents.len(), 1);
    assert!(documents[0].contains("http-forward-arena"));
}
