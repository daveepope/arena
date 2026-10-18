#[path = "../subjects.rs"]
mod subjects;

use arena::lifecycle::bus::{EventBus, Observer, Publisher};
use arena::lifecycle::{Fault, RunnableState};
use arena::playbook::lifecycle::{ScopedPlaybookEvent, ScopedPlaybookLifecycle, CHANNEL};
use arena::{Match, MatchTrait};
use std::sync::{Arc, Mutex};
use subjects::{probe_playbook, Behaviour};

const ARENA_ID: &str = "arena-under-test";

struct RecordingObserver {
    events: Mutex<Vec<ScopedPlaybookEvent>>,
}

impl RecordingObserver {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            events: Mutex::new(Vec::new()),
        })
    }

    fn states_for(&self, playbook_id: &str) -> Vec<RunnableState> {
        self.events
            .lock()
            .expect("events lock")
            .iter()
            .filter(|event| event.playbook_id == playbook_id)
            .map(|event| event.state)
            .collect()
    }

    fn faults_for(&self, playbook_id: &str) -> Vec<Fault> {
        self.events
            .lock()
            .expect("events lock")
            .iter()
            .filter(|event| event.playbook_id == playbook_id)
            .flat_map(|event| event.faults.clone())
            .collect()
    }

    fn arena_ids(&self) -> Vec<String> {
        self.events
            .lock()
            .expect("events lock")
            .iter()
            .map(|event| event.arena_id.clone())
            .collect()
    }
}

impl Observer<ScopedPlaybookEvent> for RecordingObserver {
    fn on_event(&self, event: &ScopedPlaybookEvent) {
        self.events.lock().expect("events lock").push(event.clone());
    }
}

fn attached_match(a_match: Match, observer: Arc<RecordingObserver>) -> (Match, Arc<EventBus<ScopedPlaybookEvent>>) {
    let bus: Arc<EventBus<ScopedPlaybookEvent>> = Arc::new(EventBus::new(CHANNEL));
    bus.register_observer(observer);
    let publisher: Arc<dyn Publisher<ScopedPlaybookEvent>> =
        Arc::clone(&bus) as Arc<dyn Publisher<ScopedPlaybookEvent>>;
    a_match.attach_scoped_playbook_publisher(ARENA_ID, publisher);
    (a_match, bus)
}

#[test]
fn state_newly_built_lifecycle_is_not_started() {
    let lifecycle = ScopedPlaybookLifecycle::new("seed");

    assert_eq!(lifecycle.state(), RunnableState::NotStarted);
    assert_eq!(lifecycle.faults(), Vec::new());
}

#[test]
fn report_illegal_transition_leaves_the_state_alone() {
    let lifecycle = ScopedPlaybookLifecycle::new("seed");

    lifecycle.report(RunnableState::Started);

    assert_eq!(lifecycle.state(), RunnableState::NotStarted);
}

#[test]
fn report_without_an_attachment_still_tracks_the_state() {
    let lifecycle = ScopedPlaybookLifecycle::new("seed");

    lifecycle.report(RunnableState::Starting);

    assert_eq!(lifecycle.state(), RunnableState::Starting);
}

#[tokio::test]
async fn run_playbook_scoped_run_reports_started_then_stopped_on_drop() {
    let observer = RecordingObserver::new();
    let (a_match, _bus) = attached_match(
        Match::new("healthy", vec![], vec![])
            .register_playbook(probe_playbook("reset-orders").into_playbook(), false),
        observer.clone(),
    );

    let active = a_match
        .run_playbook("reset-orders")
        .await
        .expect("playbook registered")
        .expect("playbook should run");
    assert_eq!(
        observer.states_for("reset-orders"),
        vec![RunnableState::Starting, RunnableState::Started]
    );

    drop(active);

    assert_eq!(
        observer.states_for("reset-orders"),
        vec![
            RunnableState::Starting,
            RunnableState::Started,
            RunnableState::Stopping,
            RunnableState::Stopped
        ]
    );
}

#[tokio::test]
async fn run_playbook_run_twice_reports_two_cycles() {
    let observer = RecordingObserver::new();
    let (a_match, _bus) = attached_match(
        Match::new("healthy", vec![], vec![])
            .register_playbook(probe_playbook("reset-orders").into_playbook(), false),
        observer.clone(),
    );

    for _ in 0..2 {
        let active = a_match
            .run_playbook("reset-orders")
            .await
            .expect("playbook registered")
            .expect("playbook should run");
        drop(active);
    }

    assert_eq!(
        observer.states_for("reset-orders"),
        vec![
            RunnableState::Starting,
            RunnableState::Started,
            RunnableState::Stopping,
            RunnableState::Stopped,
            RunnableState::Starting,
            RunnableState::Started,
            RunnableState::Stopping,
            RunnableState::Stopped
        ]
    );
}

#[tokio::test]
async fn run_playbook_failing_run_reports_faulted_with_its_fault() {
    let observer = RecordingObserver::new();
    let (a_match, _bus) = attached_match(
        Match::new("healthy", vec![], vec![]).register_playbook(
            probe_playbook("seed")
                .behaving(Behaviour::FailStart)
                .into_playbook(),
            false,
        ),
        observer.clone(),
    );

    let outcome = a_match
        .run_playbook("seed")
        .await
        .expect("playbook registered");

    assert!(outcome.is_err());
    assert_eq!(
        observer.states_for("seed"),
        vec![RunnableState::Starting, RunnableState::Faulted]
    );
    assert_eq!(observer.faults_for("seed").len(), 1);
}

#[tokio::test]
async fn attach_scoped_playbook_publisher_stamps_the_arena_id_on_every_event() {
    let observer = RecordingObserver::new();
    let (a_match, _bus) = attached_match(
        Match::new("healthy", vec![], vec![])
            .register_playbook(probe_playbook("reset-orders").into_playbook(), false),
        observer.clone(),
    );

    let active = a_match
        .run_playbook("reset-orders")
        .await
        .expect("playbook registered")
        .expect("playbook should run");
    drop(active);

    assert!(!observer.arena_ids().is_empty());
    assert!(observer.arena_ids().iter().all(|id| id == ARENA_ID));
}

#[tokio::test]
async fn run_playbook_unregistered_observer_receives_nothing_further() {
    let observer = RecordingObserver::new();
    let bus: Arc<EventBus<ScopedPlaybookEvent>> = Arc::new(EventBus::new(CHANNEL));
    let registration = bus.register_observer(observer.clone());
    let publisher: Arc<dyn Publisher<ScopedPlaybookEvent>> =
        Arc::clone(&bus) as Arc<dyn Publisher<ScopedPlaybookEvent>>;
    let a_match = Match::new("healthy", vec![], vec![])
        .register_playbook(probe_playbook("reset-orders").into_playbook(), false);
    a_match.attach_scoped_playbook_publisher(ARENA_ID, publisher);

    bus.unregister_observer(registration);
    let active = a_match
        .run_playbook("reset-orders")
        .await
        .expect("playbook registered")
        .expect("playbook should run");
    drop(active);

    assert_eq!(observer.states_for("reset-orders"), Vec::new());
}

#[tokio::test]
async fn start_startup_playbook_reports_starting_then_started() {
    let observer = RecordingObserver::new();
    let (mut a_match, _bus) = attached_match(
        Match::new("healthy", vec![], vec![])
            .register_playbook(probe_playbook("apply-schema").into_playbook(), true),
        observer.clone(),
    );
    let publisher = arena::lifecycle::ArenaLifecyclePublisher::new("arena-under-test", vec![]);

    a_match.start(&publisher).await.expect("match should open");

    assert_eq!(
        observer.states_for("apply-schema"),
        vec![RunnableState::Starting, RunnableState::Started]
    );
}

#[tokio::test]
async fn start_failing_startup_playbook_reports_faulted_on_its_own_row() {
    let observer = RecordingObserver::new();
    let (mut a_match, _bus) = attached_match(
        Match::new("faulting", vec![], vec![]).register_playbook(
            probe_playbook("seed")
                .behaving(Behaviour::FailStart)
                .into_playbook(),
            true,
        ),
        observer.clone(),
    );
    let publisher = arena::lifecycle::ArenaLifecyclePublisher::new("arena-under-test", vec![]);

    a_match
        .start(&publisher)
        .await
        .expect_err("match should fault");

    assert_eq!(
        observer.states_for("seed"),
        vec![RunnableState::Starting, RunnableState::Faulted]
    );
    assert_eq!(observer.faults_for("seed").len(), 1);
}

#[tokio::test]
async fn release_all_marks_a_started_playbook_stopped() {
    let observer = RecordingObserver::new();
    let (mut a_match, _bus) = attached_match(
        Match::new("healthy", vec![], vec![])
            .register_playbook(probe_playbook("apply-schema").into_playbook(), true),
        observer.clone(),
    );
    let publisher = arena::lifecycle::ArenaLifecyclePublisher::new("arena-under-test", vec![]);
    a_match.start(&publisher).await.expect("match should open");

    a_match.release_all();

    assert_eq!(
        observer.states_for("apply-schema"),
        vec![
            RunnableState::Starting,
            RunnableState::Started,
            RunnableState::Stopping,
            RunnableState::Stopped
        ]
    );
}

#[test]
fn report_terminal_from_started_reports_stopping_then_stopped() {
    let lifecycle = ScopedPlaybookLifecycle::new("reset-orders");
    lifecycle.report(RunnableState::Starting);
    lifecycle.report(RunnableState::Started);

    lifecycle.report_terminal();

    assert_eq!(lifecycle.state(), RunnableState::Stopped);
}

#[test]
fn report_terminal_from_a_handle_that_never_started_does_nothing() {
    let lifecycle = ScopedPlaybookLifecycle::new("reset-orders");

    lifecycle.report_terminal();

    assert_eq!(lifecycle.state(), RunnableState::NotStarted);
}

#[test]
fn report_terminal_called_twice_stays_stopped() {
    let lifecycle = ScopedPlaybookLifecycle::new("reset-orders");
    lifecycle.report(RunnableState::Starting);
    lifecycle.report(RunnableState::Started);

    lifecycle.report_terminal();
    lifecycle.report_terminal();

    assert_eq!(lifecycle.state(), RunnableState::Stopped);
}

#[tokio::test]
async fn run_playbook_successful_run_reports_started_exactly_once() {
    let observer = RecordingObserver::new();
    let (a_match, _bus) = attached_match(
        Match::new("healthy", vec![], vec![])
            .register_playbook(probe_playbook("reset-orders").into_playbook(), false),
        observer.clone(),
    );

    let active = a_match
        .run_playbook("reset-orders")
        .await
        .expect("playbook registered")
        .expect("playbook should run");
    drop(active);

    let started = observer
        .states_for("reset-orders")
        .into_iter()
        .filter(|state| *state == RunnableState::Started)
        .count();
    assert_eq!(started, 1);
}

#[tokio::test]
async fn run_playbook_panicking_run_reports_faulted_rather_than_sticking_at_starting() {
    let observer = RecordingObserver::new();
    let (a_match, _bus) = attached_match(
        Match::new("panicking", vec![], vec![]).register_playbook(
            probe_playbook("seed")
                .behaving(Behaviour::PanicStart)
                .into_playbook(),
            false,
        ),
        observer.clone(),
    );

    let outcome = a_match
        .run_playbook("seed")
        .await
        .expect("playbook registered");

    assert!(outcome.is_err());
    assert_eq!(
        observer.states_for("seed"),
        vec![RunnableState::Starting, RunnableState::Faulted]
    );
}

#[test]
fn exit_two_live_handles_only_reports_stopped_once_the_last_one_goes() {
    let lifecycle = ScopedPlaybookLifecycle::new("reset-orders");
    lifecycle.report_starting();
    lifecycle.report_started();
    lifecycle.report_started();

    lifecycle.exit();
    assert_eq!(lifecycle.state(), RunnableState::Started);

    lifecycle.exit();
    assert_eq!(lifecycle.state(), RunnableState::Stopped);
}

#[test]
fn report_starting_while_already_started_is_not_reported_as_illegal() {
    let observer = RecordingObserver::new();
    let bus: Arc<EventBus<ScopedPlaybookEvent>> = Arc::new(EventBus::new(CHANNEL));
    bus.register_observer(observer.clone());
    let lifecycle = ScopedPlaybookLifecycle::new("reset-orders");
    lifecycle.attach(ARENA_ID, Arc::clone(&bus) as Arc<dyn Publisher<ScopedPlaybookEvent>>);
    lifecycle.report_starting();
    lifecycle.report_started();

    lifecycle.report_starting();

    assert_eq!(
        observer.states_for("reset-orders"),
        vec![RunnableState::Starting, RunnableState::Started]
    );
}

