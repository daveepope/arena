use std::sync::Arc;
use std::time::Duration;

use arena::component::{Component, RunnableComponent};
use arena::dependency::{Dependency, RunnableDependency};
use arena::{ClosedArena, Fault, Match, MatchTrait, RunnableState};
use arena_dashboard::HttpForwardingObserver;
use rand::Rng;

fn random_step_delay() -> Duration {
    Duration::from_millis(rand::thread_rng().gen_range(100..=500))
}

struct ProbeDependency {
    identifier: String,
    state: RunnableState,
    children: Vec<Dependency>,
    component_ids: Vec<String>,
    fails_to_start: bool,
    faults: Vec<Fault>,
}

impl ProbeDependency {
    fn new(identifier: &str) -> Self {
        Self {
            identifier: identifier.to_string(),
            state: RunnableState::NotStarted,
            children: Vec::new(),
            component_ids: Vec::new(),
            fails_to_start: false,
            faults: Vec::new(),
        }
    }

    fn with_child(mut self, child: Dependency) -> Self {
        self.children.push(child);
        self
    }

    fn with_component_id(mut self, component_id: &str) -> Self {
        self.component_ids.push(component_id.to_string());
        self
    }

    fn failing_to_start(mut self) -> Self {
        self.fails_to_start = true;
        self
    }

    fn into_dependency(self) -> Dependency {
        Box::new(self)
    }
}

#[async_trait::async_trait]
impl RunnableDependency for ProbeDependency {
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
        &self.faults
    }

    async fn start(&mut self) -> Result<(), Fault> {
        self.state = RunnableState::Starting;
        for child in self.children.iter_mut() {
            arena::dependency::start_child(child).await?;
        }
        tokio::time::sleep(random_step_delay()).await;
        self.state = RunnableState::ReadinessCheck;
        tokio::time::sleep(random_step_delay()).await;
        if self.fails_to_start {
            let fault = Fault::dependency(&self.identifier, "simulated dependency start failure");
            self.state = RunnableState::Faulted;
            self.faults.push(fault.clone());
            return Err(fault);
        }
        self.state = RunnableState::Started;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        if self.state == RunnableState::Faulted {
            return Ok(());
        }
        self.state = RunnableState::Stopping;
        tokio::time::sleep(random_step_delay()).await;
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

    fn component_ids(&self) -> &[String] {
        &self.component_ids
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

struct ProbeComponent {
    identifier: String,
    state: RunnableState,
    children: Vec<Component>,
}

impl ProbeComponent {
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
impl RunnableComponent for ProbeComponent {
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
        self.state = RunnableState::Starting;
        for child in self.children.iter_mut() {
            arena::component::start_child(child).await?;
        }
        tokio::time::sleep(random_step_delay()).await;
        self.state = RunnableState::ReadinessCheck;
        tokio::time::sleep(random_step_delay()).await;
        self.state = RunnableState::Started;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        self.state = RunnableState::Stopping;
        tokio::time::sleep(random_step_delay()).await;
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

fn topology_a() -> (Vec<Dependency>, Vec<Component>) {
    let dependencies = vec![
        ProbeDependency::new("orders-db (postgres)")
            .with_component_id("orders-api")
            .into_dependency(),
        ProbeDependency::new("pricing-service (http)")
            .with_component_id("orders-api")
            .into_dependency(),
        ProbeDependency::new("orders-events (kafka)")
            .with_component_id("payments-worker")
            .into_dependency(),
        ProbeDependency::new("payments-db (postgres)")
            .with_component_id("payments-worker")
            .with_child(ProbeDependency::new("fraud-check-service (http)").into_dependency())
            .into_dependency(),
        ProbeDependency::new("refunds-queue (rabbitmq)")
            .with_component_id("payments-worker")
            .into_dependency(),
    ];
    let components = vec![ProbeComponent::new("orders-api")
        .with_child(ProbeComponent::new("payments-worker").into_component())
        .into_component()];
    (dependencies, components)
}

fn topology_c() -> (Vec<Dependency>, Vec<Component>) {
    let dependencies = vec![
        ProbeDependency::new("orders-db (postgres)")
            .with_component_id("orders-api")
            .into_dependency(),
        ProbeDependency::new("pricing-service (http)")
            .with_component_id("orders-api")
            .into_dependency(),
        ProbeDependency::new("orders-events (kafka)")
            .with_component_id("payments-worker")
            .into_dependency(),
        ProbeDependency::new("payments-db (postgres)")
            .with_component_id("payments-worker")
            .with_child(ProbeDependency::new("fraud-check-service (http)").into_dependency())
            .into_dependency(),
        ProbeDependency::new("refunds-queue (rabbitmq)")
            .with_component_id("payments-worker")
            .failing_to_start()
            .into_dependency(),
    ];
    let components = vec![ProbeComponent::new("orders-api")
        .with_child(ProbeComponent::new("payments-worker").into_component())
        .into_component()];
    (dependencies, components)
}

fn topology_b() -> (Vec<Dependency>, Vec<Component>) {
    let dependencies = vec![ProbeDependency::new("catalog-db (postgres)")
        .with_component_id("catalog-service")
        .with_component_id("search-service")
        .with_component_id("inventory-service")
        .into_dependency()];
    let components = vec![
        ProbeComponent::new("catalog-service").into_component(),
        ProbeComponent::new("search-service").into_component(),
        ProbeComponent::new("inventory-service").into_component(),
    ];
    (dependencies, components)
}

fn closed_arena_for(
    arena_id: &str,
    dashboard_url: &str,
    dependencies: Vec<Dependency>,
    components: Vec<Component>,
) -> ClosedArena {
    let matches: Vec<Box<dyn MatchTrait>> =
        vec![Box::new(Match::new(arena_id, dependencies, components))];
    ClosedArena::new(arena_id.to_string(), matches)
        .observe(Arc::new(HttpForwardingObserver::new(dashboard_url)))
}

#[tokio::main]
async fn main() {
    let dashboard_url = std::env::var("ARENA_DASHBOARD_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:4873".to_string());

    let (deps_a, comps_a) = topology_a();
    let (deps_b, comps_b) = topology_b();
    let (deps_c, comps_c) = topology_c();
    let closed_a = closed_arena_for("arena-orders", &dashboard_url, deps_a, comps_a);
    let closed_b = closed_arena_for("arena-catalog", &dashboard_url, deps_b, comps_b);
    let closed_c = closed_arena_for("arena-orders-faulted", &dashboard_url, deps_c, comps_c);

    println!("posting demo lifecycle events for 3 arenas to {dashboard_url}");

    let open_a = closed_a.open().await.expect("arena-orders should open");
    let open_b = closed_b.open().await.expect("arena-catalog should open");
    match closed_c.open().await {
        Ok(_) => panic!("arena-orders-faulted was expected to fault on start"),
        Err(state) => println!("arena-orders-faulted faulted as expected: {state}"),
    }

    println!("\nAll arenas open. Press enter to tear down...");
    let _ = tokio::task::spawn_blocking(|| {
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
    })
    .await;

    if let Err(state) = open_a.close().await {
        eprintln!("arena-orders closed with faults: {state}");
    }
    if let Err(state) = open_b.close().await {
        eprintln!("arena-catalog closed with faults: {state}");
    }

    println!("\nTeardown complete.");
}
