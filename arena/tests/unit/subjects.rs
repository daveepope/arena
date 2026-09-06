#![allow(dead_code)]

use arena::component::{Component, RunnableComponent};
use arena::dependency::{Dependency, RunnableDependency};
use arena::lifecycle::{
    ArenaLifecycleObserver, ArenaLifecycleState, ArenaState, Fault, RunnableState,
};
use arena::playbook::{ActivePlaybook, Playbook};
use async_trait::async_trait;
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Behaviour {
    #[default]
    Healthy,
    FailStart,
    PanicStart,
    FailStop,
    PanicStop,
    PanicForceStop,
    ResistTeardown,
    FaultSilently,
    ReportStoppedWithoutStopping,
}

#[derive(Default)]
pub struct CallCounts {
    start: AtomicUsize,
    stop: AtomicUsize,
    force_stop: AtomicUsize,
    release: AtomicUsize,
}

impl CallCounts {
    pub fn starts(&self) -> usize {
        self.start.load(Ordering::SeqCst)
    }

    pub fn stops(&self) -> usize {
        self.stop.load(Ordering::SeqCst)
    }

    pub fn force_stops(&self) -> usize {
        self.force_stop.load(Ordering::SeqCst)
    }

    pub fn releases(&self) -> usize {
        self.release.load(Ordering::SeqCst)
    }
}

pub struct ProbeDependency {
    identifier: String,
    state: RunnableState,
    faults: Vec<Fault>,
    children: Vec<Dependency>,
    behaviour: Behaviour,
    counts: Arc<CallCounts>,
    order: Arc<Mutex<Vec<String>>>,
}

pub fn probe_dependency(identifier: &str) -> ProbeDependency {
    ProbeDependency {
        identifier: identifier.to_string(),
        state: RunnableState::NotStarted,
        faults: Vec::new(),
        children: Vec::new(),
        behaviour: Behaviour::Healthy,
        counts: Arc::new(CallCounts::default()),
        order: Arc::new(Mutex::new(Vec::new())),
    }
}

impl ProbeDependency {
    pub fn behaving(mut self, behaviour: Behaviour) -> Self {
        self.behaviour = behaviour;
        self
    }

    pub fn recording_order(mut self, order: Arc<Mutex<Vec<String>>>) -> Self {
        self.order = order;
        self
    }

    pub fn with_child(mut self, child: Dependency) -> Self {
        self.children.push(child);
        self
    }

    pub fn counts(&self) -> Arc<CallCounts> {
        Arc::clone(&self.counts)
    }

    pub fn into_dependency(self) -> Dependency {
        Box::new(self)
    }

    fn note(&self, what: &str) {
        self.order
            .lock()
            .expect("order lock")
            .push(format!("{}:{what}", self.identifier));
    }
}

#[async_trait]
impl RunnableDependency for ProbeDependency {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn state(&self) -> RunnableState {
        if self.behaviour == Behaviour::ReportStoppedWithoutStopping {
            return RunnableState::Stopped;
        }
        self.state
    }

    fn faults(&self) -> &[Fault] {
        &self.faults
    }

    async fn start(&mut self) -> Result<(), Fault> {
        self.counts.start.fetch_add(1, Ordering::SeqCst);
        self.note("start");
        self.state = RunnableState::Starting;
        if self.behaviour == Behaviour::PanicStart {
            panic!("dependency '{}' start failed", self.identifier);
        }
        self.state = RunnableState::ReadinessCheck;
        if self.behaviour == Behaviour::FailStart {
            let fault = Fault::dependency(&self.identifier, "readiness check never passed");
            self.faults.push(fault.clone());
            self.state = RunnableState::Stopped;
            return Err(fault);
        }
        self.state = RunnableState::Started;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        self.counts.stop.fetch_add(1, Ordering::SeqCst);
        self.note("stop");
        self.state = RunnableState::Stopping;
        if self.behaviour == Behaviour::PanicStop {
            panic!("dependency '{}' stop failed", self.identifier);
        }
        if matches!(self.behaviour, Behaviour::FailStop | Behaviour::ResistTeardown) {
            let fault = Fault::dependency(&self.identifier, "stop did not complete");
            self.faults.push(fault.clone());
            self.state = RunnableState::Faulted;
            return Err(fault);
        }
        self.state = RunnableState::Stopped;
        Ok(())
    }

    fn release(&mut self) {
        self.counts.release.fetch_add(1, Ordering::SeqCst);
        self.note("release");
        for child in self.children.iter_mut() {
            child.release();
        }
        self.state = RunnableState::Stopped;
    }

    async fn force_stop(&mut self) {
        self.counts.force_stop.fetch_add(1, Ordering::SeqCst);
        self.note("force_stop");
        for child in self.children.iter_mut() {
            child.force_stop().await;
        }
        if self.behaviour == Behaviour::PanicForceStop {
            panic!("dependency '{}' forced teardown failed", self.identifier);
        }
        if self.behaviour == Behaviour::FaultSilently {
            self.state = RunnableState::Faulted;
            return;
        }
        if self.behaviour == Behaviour::ResistTeardown {
            if self.faults.is_empty() {
                self.faults.push(Fault::dependency(
                    &self.identifier,
                    "forced teardown could not confirm removal",
                ));
            }
            self.state = RunnableState::Faulted;
            return;
        }
        self.state = RunnableState::Stopped;
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
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

pub struct ProbeComponent {
    identifier: String,
    state: RunnableState,
    faults: Vec<Fault>,
    children: Vec<Component>,
    behaviour: Behaviour,
    counts: Arc<CallCounts>,
    order: Arc<Mutex<Vec<String>>>,
}

pub fn probe_component(identifier: &str) -> ProbeComponent {
    ProbeComponent {
        identifier: identifier.to_string(),
        state: RunnableState::NotStarted,
        faults: Vec::new(),
        children: Vec::new(),
        behaviour: Behaviour::Healthy,
        counts: Arc::new(CallCounts::default()),
        order: Arc::new(Mutex::new(Vec::new())),
    }
}

impl ProbeComponent {
    pub fn behaving(mut self, behaviour: Behaviour) -> Self {
        self.behaviour = behaviour;
        self
    }

    pub fn recording_order(mut self, order: Arc<Mutex<Vec<String>>>) -> Self {
        self.order = order;
        self
    }

    pub fn with_child(mut self, child: Component) -> Self {
        self.children.push(child);
        self
    }

    pub fn counts(&self) -> Arc<CallCounts> {
        Arc::clone(&self.counts)
    }

    pub fn into_component(self) -> Component {
        Box::new(self)
    }

    fn note(&self, what: &str) {
        self.order
            .lock()
            .expect("order lock")
            .push(format!("{}:{what}", self.identifier));
    }
}

#[async_trait]
impl RunnableComponent for ProbeComponent {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn state(&self) -> RunnableState {
        if self.behaviour == Behaviour::ReportStoppedWithoutStopping {
            return RunnableState::Stopped;
        }
        self.state
    }

    fn faults(&self) -> &[Fault] {
        &self.faults
    }

    async fn start(&mut self) -> Result<(), Fault> {
        self.counts.start.fetch_add(1, Ordering::SeqCst);
        self.note("start");
        self.state = RunnableState::Starting;
        if self.behaviour == Behaviour::PanicStart {
            panic!("component '{}' start failed", self.identifier);
        }
        self.state = RunnableState::ReadinessCheck;
        if self.behaviour == Behaviour::FailStart {
            let fault = Fault::component(&self.identifier, "readiness check never passed");
            self.faults.push(fault.clone());
            self.state = RunnableState::Stopped;
            return Err(fault);
        }
        self.state = RunnableState::Started;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        self.counts.stop.fetch_add(1, Ordering::SeqCst);
        self.note("stop");
        self.state = RunnableState::Stopping;
        if self.behaviour == Behaviour::PanicStop {
            panic!("component '{}' stop failed", self.identifier);
        }
        if matches!(self.behaviour, Behaviour::FailStop | Behaviour::ResistTeardown) {
            let fault = Fault::component(&self.identifier, "stop did not complete");
            self.faults.push(fault.clone());
            self.state = RunnableState::Faulted;
            return Err(fault);
        }
        self.state = RunnableState::Stopped;
        Ok(())
    }

    fn release(&mut self) {
        self.counts.release.fetch_add(1, Ordering::SeqCst);
        self.note("release");
        for child in self.children.iter_mut() {
            child.release();
        }
        self.state = RunnableState::Stopped;
    }

    async fn force_stop(&mut self) {
        self.counts.force_stop.fetch_add(1, Ordering::SeqCst);
        self.note("force_stop");
        for child in self.children.iter_mut() {
            child.force_stop().await;
        }
        if self.behaviour == Behaviour::PanicForceStop {
            panic!("component '{}' forced teardown failed", self.identifier);
        }
        if self.behaviour == Behaviour::FaultSilently {
            self.state = RunnableState::Faulted;
            return;
        }
        if self.behaviour == Behaviour::ResistTeardown {
            if self.faults.is_empty() {
                self.faults.push(Fault::component(
                    &self.identifier,
                    "forced teardown could not confirm removal",
                ));
            }
            self.state = RunnableState::Faulted;
            return;
        }
        self.state = RunnableState::Stopped;
    }

    fn add_child(&mut self, child: Box<dyn RunnableComponent>) {
        self.children.push(child);
    }

    fn children(&self) -> &[Component] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Component] {
        &mut self.children
    }
}

pub struct ProbeActivePlaybook {
    identifier: String,
}

impl ActivePlaybook for ProbeActivePlaybook {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct ProbePlaybook {
    identifier: String,
    behaviour: Behaviour,
}

pub fn probe_playbook(identifier: &str) -> ProbePlaybook {
    ProbePlaybook {
        identifier: identifier.to_string(),
        behaviour: Behaviour::Healthy,
    }
}

impl ProbePlaybook {
    pub fn behaving(mut self, behaviour: Behaviour) -> Self {
        self.behaviour = behaviour;
        self
    }

    pub fn into_playbook(self) -> Box<dyn Playbook> {
        Box::new(self)
    }
}

#[async_trait]
impl Playbook for ProbePlaybook {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    async fn run(&self, _dependencies: &[Dependency]) -> Result<Box<dyn ActivePlaybook>, Fault> {
        match self.behaviour {
            Behaviour::PanicStart => panic!("playbook '{}' run failed", self.identifier),
            Behaviour::FailStart => Err(Fault::playbook(&self.identifier, "seed data rejected")),
            _ => Ok(Box::new(ProbeActivePlaybook {
                identifier: self.identifier.clone(),
            })),
        }
    }
}

#[derive(Default)]
pub struct StateRecorder {
    snapshots: Mutex<Vec<ArenaState>>,
}

impl StateRecorder {
    pub fn states(&self) -> Vec<ArenaLifecycleState> {
        self.snapshots
            .lock()
            .expect("snapshot lock")
            .iter()
            .map(|s| s.state)
            .collect()
    }

    pub fn snapshots(&self) -> Vec<ArenaState> {
        self.snapshots.lock().expect("snapshot lock").clone()
    }

    pub fn last(&self) -> Option<ArenaState> {
        self.snapshots.lock().expect("snapshot lock").last().cloned()
    }
}

impl ArenaLifecycleObserver for StateRecorder {
    fn on_state(&self, state: &ArenaState) {
        self.snapshots
            .lock()
            .expect("snapshot lock")
            .push(state.clone());
    }
}

pub struct PanickingObserver {
    panic_on: ArenaLifecycleState,
    seen: Mutex<Vec<ArenaLifecycleState>>,
}

impl PanickingObserver {
    pub fn panicking_on(state: ArenaLifecycleState) -> Self {
        Self {
            panic_on: state,
            seen: Mutex::new(Vec::new()),
        }
    }

    pub fn seen(&self) -> Vec<ArenaLifecycleState> {
        self.seen.lock().expect("seen lock").clone()
    }
}

impl ArenaLifecycleObserver for PanickingObserver {
    fn on_state(&self, state: &ArenaState) {
        self.seen.lock().expect("seen lock").push(state.state);
        if state.state == self.panic_on {
            panic!("observer panicked on {}", state.state);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedEvent {
    pub message: String,
    pub scope: Vec<String>,
}

impl RecordedEvent {
    pub fn subject(&self) -> Option<&str> {
        self.scope
            .iter()
            .find(|entry| entry.starts_with("subject:"))
            .map(|entry| &entry["subject:".len()..])
    }

    pub fn arena(&self) -> Option<&str> {
        self.scope
            .iter()
            .find(|entry| entry.starts_with("arena:"))
            .map(|entry| &entry["arena:".len()..])
    }
}

#[derive(Default)]
pub struct RecordedEvents {
    events: Mutex<Vec<RecordedEvent>>,
}

impl RecordedEvents {
    pub fn push(&self, event: RecordedEvent) {
        self.events.lock().expect("recorded events lock").push(event);
    }

    pub fn all(&self) -> Vec<RecordedEvent> {
        self.events.lock().expect("recorded events lock").clone()
    }

    pub fn with_message(&self, needle: &str) -> Vec<RecordedEvent> {
        self.all()
            .into_iter()
            .filter(|event| event.message.contains(needle))
            .collect()
    }
}

pub struct EventScopeRecordingLayer(pub Arc<RecordedEvents>);

#[derive(Default)]
struct MessageCollector {
    message: String,
}

impl tracing::field::Visit for MessageCollector {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

#[derive(Default)]
struct SpanIdentity {
    kind: Option<String>,
    id: Option<String>,
    arena_id: Option<String>,
}

impl tracing::field::Visit for SpanIdentity {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.assign(field.name(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.assign(field.name(), value.to_string());
    }
}

impl SpanIdentity {
    fn assign(&mut self, name: &str, value: String) {
        match name {
            "arena.subject.kind" => self.kind = Some(value),
            "arena.subject.id" => self.id = Some(value),
            "arena.id" => self.arena_id = Some(value),
            _ => {}
        }
    }
}

impl<S> tracing_subscriber::Layer<S> for EventScopeRecordingLayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut identity = SpanIdentity::default();
        attrs.record(&mut identity);
        let rendered = match attrs.metadata().name() {
            "arena" => format!("arena:{}", identity.arena_id.unwrap_or_default()),
            "subject" => format!(
                "subject:{}.{}",
                identity.kind.unwrap_or_default(),
                identity.id.unwrap_or_default()
            ),
            other => other.to_string(),
        };
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(rendered);
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut collector = MessageCollector::default();
        event.record(&mut collector);
        let mut scope = Vec::new();
        if let Some(spans) = ctx.event_scope(event) {
            for span in spans {
                if let Some(rendered) = span.extensions().get::<String>() {
                    scope.push(rendered.clone());
                }
            }
        }
        self.0.push(RecordedEvent {
            message: collector.message,
            scope,
        });
    }
}
