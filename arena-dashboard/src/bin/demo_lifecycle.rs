use std::sync::Arc;
use std::time::Duration;

use arena::component::{Component, RunnableComponent};
use arena::dependency::{Dependency, RunnableDependency};
use arena::{ClosedArena, Fault, Match, MatchTrait, RunnableState};
use arena_dashboard::HttpForwardingObserver;
use arena_profile::observer::{register_observer, report, report_complete, unregister_observer};
use rand::Rng;

static STEP_DELAY_CEILING_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(300);

fn random_step_delay() -> Duration {
    let ceiling = STEP_DELAY_CEILING_MS.load(std::sync::atomic::Ordering::Relaxed);
    let floor = (ceiling / 2).max(1).min(ceiling);
    Duration::from_millis(rand::thread_rng().gen_range(floor..=ceiling))
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
    profiled: bool,
}

impl ProbeComponent {
    fn new(identifier: &str) -> Self {
        Self {
            identifier: identifier.to_string(),
            state: RunnableState::NotStarted,
            children: Vec::new(),
            profiled: false,
        }
    }

    fn with_child(mut self, child: Component) -> Self {
        self.children.push(child);
        self
    }

    fn with_cpu_profile(mut self) -> Self {
        self.profiled = true;
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
        if self.profiled {
            report(&self.identifier, RunnableState::Starting);
        }
        tokio::time::sleep(random_step_delay()).await;
        self.state = RunnableState::ReadinessCheck;
        tokio::time::sleep(random_step_delay()).await;
        self.state = RunnableState::Started;
        if self.profiled {
            report(&self.identifier, RunnableState::Started);
        }
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        self.state = RunnableState::Stopping;
        if self.profiled {
            report(&self.identifier, RunnableState::Stopping);
        }
        tokio::time::sleep(random_step_delay()).await;
        self.state = RunnableState::Stopped;
        if self.profiled {
            report_complete(&self.identifier, &folded_profile_for(&self.identifier));
        }
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
        .with_cpu_profile()
        .with_child(ProbeComponent::new("payments-worker").into_component())
        .into_component()];
    (dependencies, components)
}

fn topology_c() -> (Vec<Dependency>, Vec<Component>) {
    let dependencies = vec![
        ProbeDependency::new("orders-db-canary (postgres)")
            .with_component_id("orders-api-canary")
            .into_dependency(),
        ProbeDependency::new("pricing-service-canary (http)")
            .with_component_id("orders-api-canary")
            .into_dependency(),
        ProbeDependency::new("orders-events-canary (kafka)")
            .with_component_id("payments-worker-canary")
            .into_dependency(),
        ProbeDependency::new("payments-db-canary (postgres)")
            .with_component_id("payments-worker-canary")
            .with_child(ProbeDependency::new("fraud-check-service-canary (http)").into_dependency())
            .into_dependency(),
        ProbeDependency::new("refunds-queue-canary (rabbitmq)")
            .with_component_id("payments-worker-canary")
            .failing_to_start()
            .into_dependency(),
    ];
    let components = vec![ProbeComponent::new("orders-api-canary")
        .with_child(ProbeComponent::new("payments-worker-canary").into_component())
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
        ProbeComponent::new("catalog-service")
            .with_cpu_profile()
            .into_component(),
        ProbeComponent::new("search-service")
            .with_cpu_profile()
            .into_component(),
        ProbeComponent::new("inventory-service").into_component(),
    ];
    (dependencies, components)
}

const ORDERS_API_PROFILE: &str = "\
orders-api;main;tokio::runtime::Runtime::block_on;hyper::server::conn::serve;axum::routing::Router::call;middleware::trace;middleware::authenticate;jsonwebtoken::decode;ring::hmac::verify 18
orders-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::create_order;orders::service::place_order;orders::pricing::recalculate_totals;futures::executor::block_on;orders::pricing::fetch_fx_rate;reqwest::blocking::Client::execute;std::net::TcpStream::read 420
orders-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::create_order;orders::service::place_order;orders::pricing::recalculate_totals;futures::executor::block_on;orders::pricing::fetch_fx_rate;reqwest::blocking::Client::execute;rustls::client::handshake 96
orders-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::create_order;orders::service::place_order;orders::validation::validate_line_items;regex::Regex::new;regex_syntax::parse 168
orders-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::create_order;orders::service::place_order;orders::validation::validate_line_items;regex::Regex::is_match 24
orders-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::create_order;orders::service::place_order;orders::repository::insert_order;sqlx::query::execute;sqlx::postgres::protocol::flush;std::io::Write::write_all 54
orders-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::list_orders;orders::repository::load_orders;orders::repository::load_customer_for_each;sqlx::query::fetch_one;sqlx::postgres::protocol::flush 132
orders-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::list_orders;orders::repository::load_orders;serde_json::ser::to_writer;serde::ser::Serialize::serialize 61
orders-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::list_orders;orders::repository::load_orders;sqlx::pool::acquire;parking_lot::Mutex::lock 37
orders-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::get_order;orders::repository::load_order;sqlx::query::fetch_optional 22
orders-api;main;tokio::runtime::worker;hyper::server::conn::serve;hyper::proto::h1::Conn::read_head;bytes::BytesMut::reserve 16
orders-api;main;tokio::runtime::worker;tokio::net::TcpListener::accept 11
orders-api;main;tokio::runtime::blocking::pool::worker;orders::audit::write_audit_log;std::fs::File::write_all 29
orders-api;main;metrics::exporter::prometheus::scrape;metrics::registry::snapshot 14
orders-api;main;tokio::time::driver::park 9
";

const CATALOG_SERVICE_PROFILE: &str = "\
catalog-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::search;catalog::search::execute_query;catalog::search::rank_results;catalog::search::score_document;catalog::text::normalize;regex::Regex::captures 212
catalog-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::search;catalog::search::execute_query;catalog::search::rank_results;catalog::search::dedupe_by_title;alloc::vec::Vec::contains 164
catalog-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::search;catalog::search::execute_query;tantivy::query::BooleanQuery::scorer;tantivy::postings::SegmentPostings::decode 118
catalog-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::search;catalog::search::execute_query;catalog::cache::get_or_insert;std::collections::HashMap::insert 26
catalog-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::facets;catalog::facets::aggregate;catalog::facets::bucket_by_attribute;std::collections::BTreeMap::entry 74
catalog-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::facets;catalog::facets::aggregate;serde_json::ser::to_writer 31
catalog-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::suggest;catalog::suggest::prefix_scan;fst::Map::range 44
catalog-service;main;tokio::runtime::blocking::pool::worker;catalog::index::refresh_segments;std::fs::read;std::io::Read::read_to_end 58
catalog-service;main;tokio::runtime::blocking::pool::worker;catalog::index::refresh_segments;tantivy::directory::MmapDirectory::open_read 19
catalog-service;main;tokio::runtime::worker;hyper::proto::h1::Conn::read_head;bytes::BytesMut::reserve 13
catalog-service;main;tokio::runtime::worker;tokio::net::TcpListener::accept 9
catalog-service;main;metrics::exporter::prometheus::scrape 7
";

fn folded_profile_for(component_id: &str) -> String {
    match component_id {
        "orders-api" => ORDERS_API_PROFILE.to_string(),
        "search-service" => ORDERS_API_PROFILE.to_string(),
        "catalog-service" => CATALOG_SERVICE_PROFILE.to_string(),
        _ => format!("{component_id};main 1\n"),
    }
}

fn closed_arena_for(
    arena_id: &str,
    observer: Arc<HttpForwardingObserver>,
    dependencies: Vec<Dependency>,
    components: Vec<Component>,
) -> ClosedArena {
    let matches: Vec<Box<dyn MatchTrait>> =
        vec![Box::new(Match::new(arena_id, dependencies, components))];
    ClosedArena::new(arena_id.to_string(), matches).observe(observer)
}

#[tokio::main]
async fn main() {
    let dashboard_url = std::env::var("ARENA_DASHBOARD_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:4873".to_string());
    if let Some(ceiling) = std::env::args().nth(1).and_then(|arg| arg.parse::<u64>().ok()) {
        STEP_DELAY_CEILING_MS.store(ceiling.max(1), std::sync::atomic::Ordering::Relaxed);
    }

    let (deps_a, comps_a) = topology_a();
    let (deps_b, comps_b) = topology_b();
    let (deps_c, comps_c) = topology_c();
    let observer = Arc::new(HttpForwardingObserver::new(&dashboard_url));
    let profile_token = register_observer(observer.clone());
    let closed_a = closed_arena_for("arena-orders", observer.clone(), deps_a, comps_a);
    let closed_b = closed_arena_for("arena-catalog", observer.clone(), deps_b, comps_b);
    let closed_c = closed_arena_for("arena-orders-faulted", observer.clone(), deps_c, comps_c);

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

    unregister_observer(profile_token);

    println!("\nTeardown complete.");
}
