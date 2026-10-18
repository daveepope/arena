use std::sync::Arc;
use std::time::Duration;

use arena::component::{Component, RunnableComponent};
use arena::dependency::{Dependency, RunnableDependency};
use arena::{ClosedArena, Fault, Match, MatchTrait, RunnableState};
use arena_dashboard::{HttpForwardingObserver, ScopedPlaybookForwarder};
use arena::playbook::{ActivePlaybook, Playbook, ScopedPlaybookLifecycle};
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


struct ProbePlaybook {
    identifier: String,
    lifecycle: Arc<ScopedPlaybookLifecycle>,
    fails: bool,
}

impl ProbePlaybook {
    fn new(identifier: &str) -> Self {
        Self {
            identifier: identifier.to_string(),
            lifecycle: ScopedPlaybookLifecycle::new(identifier),
            fails: false,
        }
    }

    fn failing(mut self) -> Self {
        self.fails = true;
        self
    }

    fn into_playbook(self) -> Box<dyn Playbook> {
        Box::new(self)
    }
}

#[async_trait::async_trait]
impl Playbook for ProbePlaybook {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn lifecycle(&self) -> &Arc<ScopedPlaybookLifecycle> {
        &self.lifecycle
    }

    async fn run(&self, _dependencies: &[Dependency]) -> Result<Box<dyn ActivePlaybook>, Fault> {
        self.lifecycle.report_starting();
        tokio::time::sleep(random_step_delay()).await;
        if self.fails {
            return Err(Fault::playbook(&self.identifier, "seed data rejected"));
        }
        self.lifecycle.report_started();
        Ok(Box::new(ProbeActivePlaybook {
            identifier: self.identifier.clone(),
            lifecycle: Arc::clone(&self.lifecycle),
        }))
    }
}

struct ProbeActivePlaybook {
    identifier: String,
    lifecycle: Arc<ScopedPlaybookLifecycle>,
}

impl ActivePlaybook for ProbeActivePlaybook {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn lifecycle(&self) -> &Arc<ScopedPlaybookLifecycle> {
        &self.lifecycle
    }
}

impl Drop for ProbeActivePlaybook {
    fn drop(&mut self) {
        self.lifecycle.exit();
    }
}

fn topology_a() -> (Vec<Dependency>, Vec<Component>) {
    let dependencies = vec![
        ProbeDependency::new("duchess-db (postgres)")
            .with_component_id("cheshire-api")
            .into_dependency(),
        ProbeDependency::new("tweedledee-service (http)")
            .with_component_id("cheshire-api")
            .into_dependency(),
        ProbeDependency::new("rabbit-hole-events (kafka)")
            .with_component_id("dormouse-worker")
            .into_dependency(),
        ProbeDependency::new("queen-of-hearts-db (postgres)")
            .with_component_id("dormouse-worker")
            .with_child(ProbeDependency::new("mad-hatter-service (http)").into_dependency())
            .into_dependency(),
        ProbeDependency::new("white-rabbit-queue (rabbitmq)")
            .with_component_id("dormouse-worker")
            .into_dependency(),
    ];
    let components = vec![ProbeComponent::new("cheshire-api")
        .with_cpu_profile()
        .with_child(ProbeComponent::new("dormouse-worker").into_component())
        .into_component()];
    (dependencies, components)
}

fn topology_d() -> (Vec<Dependency>, Vec<Component>) {
    let dependencies = vec![ProbeDependency::new("croquet-db (postgres)")
        .with_component_id("flamingo-service")
        .into_dependency()];
    let components = vec![ProbeComponent::new("flamingo-service").into_component()];
    (dependencies, components)
}

fn topology_c() -> (Vec<Dependency>, Vec<Component>) {
    let dependencies = vec![
        ProbeDependency::new("looking-glass-db (postgres)")
            .with_component_id("jabberwock-api")
            .into_dependency(),
        ProbeDependency::new("tweedledum-service (http)")
            .with_component_id("jabberwock-api")
            .into_dependency(),
        ProbeDependency::new("bandersnatch-events (kafka)")
            .with_component_id("bread-and-butterfly-worker")
            .into_dependency(),
        ProbeDependency::new("red-queen-db (postgres)")
            .with_component_id("bread-and-butterfly-worker")
            .with_child(ProbeDependency::new("walrus-service (http)").into_dependency())
            .into_dependency(),
        ProbeDependency::new("oyster-queue (rabbitmq)")
            .with_component_id("bread-and-butterfly-worker")
            .failing_to_start()
            .into_dependency(),
    ];
    let components = vec![ProbeComponent::new("jabberwock-api")
        .with_child(ProbeComponent::new("bread-and-butterfly-worker").into_component())
        .into_component()];
    (dependencies, components)
}

fn topology_b() -> (Vec<Dependency>, Vec<Component>) {
    let dependencies = vec![ProbeDependency::new("wonderland-db (postgres)")
        .with_component_id("caterpillar-service")
        .with_component_id("gryphon-service")
        .with_component_id("dodo-service")
        .into_dependency()];
    let components = vec![
        ProbeComponent::new("caterpillar-service")
            .with_cpu_profile()
            .into_component(),
        ProbeComponent::new("gryphon-service")
            .with_cpu_profile()
            .into_component(),
        ProbeComponent::new("dodo-service").into_component(),
    ];
    (dependencies, components)
}

const CHESHIRE_API_PROFILE: &str = "\
cheshire-api;main;tokio::runtime::Runtime::block_on;hyper::server::conn::serve;axum::routing::Router::call;middleware::trace;middleware::authenticate;jsonwebtoken::decode;ring::hmac::verify 18
cheshire-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::create_order;orders::service::place_order;orders::pricing::recalculate_totals;futures::executor::block_on;orders::pricing::fetch_fx_rate;reqwest::blocking::Client::execute;std::net::TcpStream::read 420
cheshire-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::create_order;orders::service::place_order;orders::pricing::recalculate_totals;futures::executor::block_on;orders::pricing::fetch_fx_rate;reqwest::blocking::Client::execute;rustls::client::handshake 96
cheshire-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::create_order;orders::service::place_order;orders::validation::validate_line_items;regex::Regex::new;regex_syntax::parse 168
cheshire-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::create_order;orders::service::place_order;orders::validation::validate_line_items;regex::Regex::is_match 24
cheshire-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::create_order;orders::service::place_order;orders::repository::insert_order;sqlx::query::execute;sqlx::postgres::protocol::flush;std::io::Write::write_all 54
cheshire-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::list_orders;orders::repository::load_orders;orders::repository::load_customer_for_each;sqlx::query::fetch_one;sqlx::postgres::protocol::flush 132
cheshire-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::list_orders;orders::repository::load_orders;serde_json::ser::to_writer;serde::ser::Serialize::serialize 61
cheshire-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::list_orders;orders::repository::load_orders;sqlx::pool::acquire;parking_lot::Mutex::lock 37
cheshire-api;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::get_order;orders::repository::load_order;sqlx::query::fetch_optional 22
cheshire-api;main;tokio::runtime::worker;hyper::server::conn::serve;hyper::proto::h1::Conn::read_head;bytes::BytesMut::reserve 16
cheshire-api;main;tokio::runtime::worker;tokio::net::TcpListener::accept 11
cheshire-api;main;tokio::runtime::blocking::pool::worker;orders::audit::write_audit_log;std::fs::File::write_all 29
cheshire-api;main;metrics::exporter::prometheus::scrape;metrics::registry::snapshot 14
cheshire-api;main;tokio::time::driver::park 9
";

const CATERPILLAR_SERVICE_PROFILE: &str = "\
caterpillar-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::search;catalog::search::execute_query;catalog::search::rank_results;catalog::search::score_document;catalog::text::normalize;regex::Regex::captures 212
caterpillar-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::search;catalog::search::execute_query;catalog::search::rank_results;catalog::search::dedupe_by_title;alloc::vec::Vec::contains 164
caterpillar-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::search;catalog::search::execute_query;tantivy::query::BooleanQuery::scorer;tantivy::postings::SegmentPostings::decode 118
caterpillar-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::search;catalog::search::execute_query;catalog::cache::get_or_insert;std::collections::HashMap::insert 26
caterpillar-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::facets;catalog::facets::aggregate;catalog::facets::bucket_by_attribute;std::collections::BTreeMap::entry 74
caterpillar-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::facets;catalog::facets::aggregate;serde_json::ser::to_writer 31
caterpillar-service;main;tokio::runtime::worker;hyper::server::conn::serve;axum::routing::Router::call;handlers::suggest;catalog::suggest::prefix_scan;fst::Map::range 44
caterpillar-service;main;tokio::runtime::blocking::pool::worker;catalog::index::refresh_segments;std::fs::read;std::io::Read::read_to_end 58
caterpillar-service;main;tokio::runtime::blocking::pool::worker;catalog::index::refresh_segments;tantivy::directory::MmapDirectory::open_read 19
caterpillar-service;main;tokio::runtime::worker;hyper::proto::h1::Conn::read_head;bytes::BytesMut::reserve 13
caterpillar-service;main;tokio::runtime::worker;tokio::net::TcpListener::accept 9
caterpillar-service;main;metrics::exporter::prometheus::scrape 7
";

fn folded_profile_for(component_id: &str) -> String {
    match component_id {
        "cheshire-api" => CHESHIRE_API_PROFILE.to_string(),
        "gryphon-service" => CATERPILLAR_SERVICE_PROFILE.to_string(),
        "caterpillar-service" => CATERPILLAR_SERVICE_PROFILE.to_string(),
        _ => format!("{component_id};main 1\n"),
    }
}

fn scoped_playbooks_for(arena_id: &str) -> Vec<&'static str> {
    match arena_id {
        "arena-tea-party" => vec!["reset-tea-party-db", "hatter-gateway-outage"],
        "arena-rabbit-hole" => vec!["reset-rabbit-hole-db", "caterpillar-cache-purge"],
        _ => vec!["reset-looking-glass-db"],
    }
}

fn closed_arena_for(
    arena_id: &str,
    observer: Arc<HttpForwardingObserver>,
    dependencies: Vec<Dependency>,
    components: Vec<Component>,
    faulting_playbook: bool,
) -> ClosedArena {
    let mut a_match = Match::new(arena_id, dependencies, components)
        .register_playbook(ProbePlaybook::new("apply-schema-migrations").into_playbook(), true)
        .register_playbook(ProbePlaybook::new("seed-reference-data").into_playbook(), true);
    for scoped in scoped_playbooks_for(arena_id) {
        a_match = a_match.register_playbook(ProbePlaybook::new(scoped).into_playbook(), false);
    }
    if faulting_playbook {
        a_match = a_match
            .register_playbook(ProbePlaybook::new("seed-croquet-data").failing().into_playbook(), true);
    }
    let matches: Vec<Box<dyn MatchTrait>> = vec![Box::new(a_match)];
    ClosedArena::new(arena_id.to_string(), matches)
        .observe(observer.clone())
        .observe_scoped_playbooks(ScopedPlaybookForwarder::new(observer))
}

async fn cycle_scoped_playbooks(arena: &arena::OpenArena, arena_id: &str) {
    for scoped in scoped_playbooks_for(arena_id) {
        let Some(outcome) = arena.run_playbook(scoped).await else {
            continue;
        };
        if let Ok(active) = outcome {
            tokio::time::sleep(random_step_delay()).await;
            drop(active);
        }
        tokio::time::sleep(random_step_delay()).await;
    }
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
    let (deps_d, comps_d) = topology_d();
    let observer = Arc::new(HttpForwardingObserver::new(&dashboard_url));
    let profile_token = register_observer(observer.clone());
    let closed_a = closed_arena_for("arena-tea-party", observer.clone(), deps_a, comps_a, false);
    let closed_b = closed_arena_for("arena-rabbit-hole", observer.clone(), deps_b, comps_b, false);
    let closed_c = closed_arena_for("arena-looking-glass", observer.clone(), deps_c, comps_c, false);
    let closed_d = closed_arena_for("arena-croquet-ground", observer.clone(), deps_d, comps_d, true);

    println!("posting demo lifecycle events for 4 arenas to {dashboard_url}");

    let open_a = closed_a.open().await.expect("arena-tea-party should open");
    let open_b = closed_b.open().await.expect("arena-rabbit-hole should open");
    match closed_c.open().await {
        Ok(_) => panic!("arena-looking-glass was expected to fault on start"),
        Err(state) => println!("arena-looking-glass faulted as expected: {state}"),
    }
    match closed_d.open().await {
        Ok(_) => panic!("arena-croquet-ground was expected to fault on its playbook"),
        Err(state) => println!("arena-croquet-ground faulted as expected: {state}"),
    }

    cycle_scoped_playbooks(&open_a, "arena-tea-party").await;
    cycle_scoped_playbooks(&open_b, "arena-rabbit-hole").await;

    println!("\nAll arenas open. Press enter to tear down...");
    let _ = tokio::task::spawn_blocking(|| {
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
    })
    .await;

    if let Err(state) = open_a.close().await {
        eprintln!("arena-tea-party closed with faults: {state}");
    }
    if let Err(state) = open_b.close().await {
        eprintln!("arena-rabbit-hole closed with faults: {state}");
    }

    unregister_observer(profile_token);

    println!("\nTeardown complete.");
}
