#[path = "../subjects.rs"]
mod subjects;

use arena::lifecycle::bus::{EventBus, Publisher};
use arena::lifecycle::Fault;
use arena::playbook::PlaybookState;
use arena::playbook::lifecycle::{
    PlaybookKind, PlaybookEvent, PlaybookLifecycle, CHANNEL,
};
use arena::{Match, MatchTrait};
use std::sync::Arc;
use subjects::{probe_playbook, Behaviour, RecordingPlaybookObserver};

const ARENA_ID: &str = "arena-under-test";

fn attached_match(a_match: Match, observer: Arc<RecordingPlaybookObserver>) -> (Match, Arc<EventBus<PlaybookEvent>>) {
    let bus: Arc<EventBus<PlaybookEvent>> = Arc::new(EventBus::new(CHANNEL));
    bus.register_observer(observer);
    let publisher: Arc<dyn Publisher<PlaybookEvent>> =
        Arc::clone(&bus) as Arc<dyn Publisher<PlaybookEvent>>;
    a_match.attach_playbook_publisher(ARENA_ID, publisher);
    (a_match, bus)
}

#[test]
fn state_newly_built_lifecycle_is_registered() {
    let lifecycle = PlaybookLifecycle::new("seed");

    assert_eq!(lifecycle.state(), PlaybookState::Registered);
    assert_eq!(lifecycle.faults(), Vec::new());
}

#[test]
fn report_illegal_transition_leaves_the_state_alone() {
    let lifecycle = PlaybookLifecycle::new("seed");

    lifecycle.report(PlaybookState::Stopped);

    assert_eq!(lifecycle.state(), PlaybookState::Registered);
}

#[test]
fn report_without_an_attachment_still_tracks_the_state() {
    let lifecycle = PlaybookLifecycle::new("seed");

    lifecycle.report(PlaybookState::Running);

    assert_eq!(lifecycle.state(), PlaybookState::Running);
}

#[tokio::test]
async fn run_playbook_run_reports_running_then_stopped_on_drop() {
    let observer = RecordingPlaybookObserver::new();
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
        vec![
            PlaybookState::Registered,
            PlaybookState::Running
        ]
    );

    drop(active);

    assert_eq!(
        observer.states_for("reset-orders"),
        vec![
            PlaybookState::Registered,
            PlaybookState::Running,
            PlaybookState::Stopped
        ]
    );
}

#[tokio::test]
async fn run_playbook_run_twice_reports_two_cycles() {
    let observer = RecordingPlaybookObserver::new();
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
            PlaybookState::Registered,
            PlaybookState::Running,
            PlaybookState::Stopped,
            PlaybookState::Running,
            PlaybookState::Stopped
        ]
    );
}

#[tokio::test]
async fn run_playbook_failing_run_reports_faulted_with_its_fault() {
    let observer = RecordingPlaybookObserver::new();
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
        vec![
            PlaybookState::Registered,
            PlaybookState::Running,
            PlaybookState::Faulted
        ]
    );
    assert_eq!(observer.faults_for("seed").len(), 1);
}

#[tokio::test]
async fn attach_playbook_publisher_stamps_the_arena_id_on_every_event() {
    let observer = RecordingPlaybookObserver::new();
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
    let observer = RecordingPlaybookObserver::new();
    let bus: Arc<EventBus<PlaybookEvent>> = Arc::new(EventBus::new(CHANNEL));
    let registration = bus.register_observer(observer.clone());
    let publisher: Arc<dyn Publisher<PlaybookEvent>> =
        Arc::clone(&bus) as Arc<dyn Publisher<PlaybookEvent>>;
    let a_match = Match::new("healthy", vec![], vec![])
        .register_playbook(probe_playbook("reset-orders").into_playbook(), false);
    bus.unregister_observer(registration);
    a_match.attach_playbook_publisher(ARENA_ID, publisher);

    let active = a_match
        .run_playbook("reset-orders")
        .await
        .expect("playbook registered")
        .expect("playbook should run");
    drop(active);

    assert_eq!(observer.states_for("reset-orders"), Vec::new());
}

#[tokio::test]
async fn start_startup_playbook_finishes_once_its_run_returns() {
    let observer = RecordingPlaybookObserver::new();
    let (mut a_match, _bus) = attached_match(
        Match::new("healthy", vec![], vec![])
            .register_playbook(probe_playbook("apply-schema").into_playbook(), true),
        observer.clone(),
    );
    let publisher = arena::lifecycle::ArenaLifecyclePublisher::new("arena-under-test", vec![]);

    a_match.start(&publisher).await.expect("match should open");

    assert_eq!(
        observer.states_for("apply-schema"),
        vec![
            PlaybookState::Registered,
            PlaybookState::Running,
            PlaybookState::Stopped
        ]
    );
}

#[tokio::test]
async fn start_failing_startup_playbook_reports_faulted_on_its_own_row() {
    let observer = RecordingPlaybookObserver::new();
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
        vec![
            PlaybookState::Registered,
            PlaybookState::Running,
            PlaybookState::Faulted
        ]
    );
    assert_eq!(observer.faults_for("seed").len(), 1);
}


#[test]
fn report_stopped_from_running_reports_stopped() {
    let lifecycle = PlaybookLifecycle::new("reset-orders");
    lifecycle.report(PlaybookState::Running);

    lifecycle.report_stopped();

    assert_eq!(lifecycle.state(), PlaybookState::Stopped);
}

#[test]
fn report_stopped_from_a_handle_that_never_ran_does_nothing() {
    let lifecycle = PlaybookLifecycle::new("reset-orders");

    lifecycle.report_stopped();

    assert_eq!(lifecycle.state(), PlaybookState::Registered);
}

#[test]
fn report_stopped_called_twice_stays_stopped() {
    let lifecycle = PlaybookLifecycle::new("reset-orders");
    lifecycle.report(PlaybookState::Running);

    lifecycle.report_stopped();
    lifecycle.report_stopped();

    assert_eq!(lifecycle.state(), PlaybookState::Stopped);
}

#[tokio::test]
async fn run_playbook_successful_run_reports_running_exactly_once() {
    let observer = RecordingPlaybookObserver::new();
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
        .filter(|state| *state == PlaybookState::Running)
        .count();
    assert_eq!(started, 1);
}

#[tokio::test]
async fn run_playbook_panicking_run_reports_faulted_rather_than_sticking_at_running() {
    let observer = RecordingPlaybookObserver::new();
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
        vec![
            PlaybookState::Registered,
            PlaybookState::Running,
            PlaybookState::Faulted
        ]
    );
}

#[test]
fn exit_two_live_handles_only_reports_stopped_once_the_last_one_goes() {
    let lifecycle = PlaybookLifecycle::new("reset-orders");
    lifecycle.report_running();
    lifecycle.enter();
    lifecycle.enter();

    lifecycle.exit();
    assert_eq!(lifecycle.state(), PlaybookState::Running);

    lifecycle.exit();
    assert_eq!(lifecycle.state(), PlaybookState::Stopped);
}

#[test]
fn report_running_while_already_running_reports_the_extra_run() {
    let observer = RecordingPlaybookObserver::new();
    let bus: Arc<EventBus<PlaybookEvent>> = Arc::new(EventBus::new(CHANNEL));
    bus.register_observer(observer.clone());
    let lifecycle = PlaybookLifecycle::new("reset-orders");
    lifecycle.attach(
        ARENA_ID,
        PlaybookKind::Scoped,
        Arc::clone(&bus) as Arc<dyn Publisher<PlaybookEvent>>,
    );
    lifecycle.report_running();

    lifecycle.report_running();

    assert_eq!(
        observer.states_for("reset-orders"),
        vec![
            PlaybookState::Registered,
            PlaybookState::Running,
            PlaybookState::Running
        ]
    );
    assert_eq!(observer.runs_for("reset-orders"), vec![0, 1, 2]);
}

#[test]
fn report_running_after_a_fault_clears_the_previous_fault() {
    let lifecycle = PlaybookLifecycle::new("reset-orders");
    lifecycle.report(PlaybookState::Running);
    lifecycle.report_fault(Fault::playbook("reset-orders", "seed data rejected"));
    assert_eq!(lifecycle.faults().len(), 1);

    lifecycle.report_running();

    assert_eq!(lifecycle.faults(), Vec::new());
    assert_eq!(lifecycle.state(), PlaybookState::Running);
}

#[tokio::test]
async fn state_read_through_the_playbook_traits_follows_the_run() {
    let playbook = probe_playbook("reset-orders").into_playbook();
    assert_eq!(playbook.state(), PlaybookState::Registered);
    assert_eq!(playbook.faults(), Vec::new());

    let active = playbook.run(&[]).await.expect("playbook should run");

    assert_eq!(active.state(), PlaybookState::Running);
    assert_eq!(active.faults(), Vec::new());
    assert_eq!(playbook.state(), PlaybookState::Running);

    drop(active);

    assert_eq!(playbook.state(), PlaybookState::Stopped);
}

#[tokio::test]
async fn faults_read_through_the_playbook_trait_carry_the_reported_fault() {
    let playbook = probe_playbook("seed")
        .behaving(Behaviour::FailStart)
        .into_playbook();

    let Err(_) = playbook.run(&[]).await else {
        panic!("playbook should fault");
    };

    assert_eq!(playbook.state(), PlaybookState::Faulted);
    assert_eq!(playbook.faults().len(), 1);
    assert_eq!(playbook.faults()[0].id, "seed");
}

#[test]
fn attach_registered_playbook_announces_its_kind_and_held_state() {
    let observer = RecordingPlaybookObserver::new();
    let (_a_match, _bus) = attached_match(
        Match::new("healthy", vec![], vec![])
            .register_playbook(probe_playbook("apply-schema").into_playbook(), true)
            .register_playbook(probe_playbook("reset-orders").into_playbook(), false),
        observer.clone(),
    );

    assert_eq!(
        observer.kinds_for("apply-schema"),
        vec![PlaybookKind::Startup]
    );
    assert_eq!(
        observer.kinds_for("reset-orders"),
        vec![PlaybookKind::Scoped]
    );
    assert_eq!(
        observer.states_for("reset-orders"),
        vec![PlaybookState::Registered]
    );
}

#[tokio::test]
async fn report_every_event_carries_the_moment_it_happened() {
    let observer = RecordingPlaybookObserver::new();
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

    let stamps = observer.timestamps_for("reset-orders");
    assert_eq!(stamps.len(), 3);
    assert!(stamps.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[tokio::test]
async fn release_all_startup_playbook_already_finished_reports_nothing_further() {
    let observer = RecordingPlaybookObserver::new();
    let (mut a_match, _bus) = attached_match(
        Match::new("healthy", vec![], vec![])
            .register_playbook(probe_playbook("apply-schema").into_playbook(), true),
        observer.clone(),
    );
    let publisher = arena::lifecycle::ArenaLifecyclePublisher::new("arena-under-test", vec![]);
    a_match.start(&publisher).await.expect("match should open");
    let reported_at_open = observer.states_for("apply-schema");

    a_match.release_all();

    assert_eq!(observer.states_for("apply-schema"), reported_at_open);
    assert_eq!(observer.runs_for("apply-schema"), vec![0, 1, 1]);
}

#[tokio::test]
async fn run_playbook_startup_playbook_rerun_by_hand_reports_a_second_cycle() {
    let observer = RecordingPlaybookObserver::new();
    let (mut a_match, _bus) = attached_match(
        Match::new("healthy", vec![], vec![])
            .register_playbook(probe_playbook("apply-schema").into_playbook(), true),
        observer.clone(),
    );
    let publisher = arena::lifecycle::ArenaLifecyclePublisher::new("arena-under-test", vec![]);
    a_match.start(&publisher).await.expect("match should open");

    let active = a_match
        .run_playbook("apply-schema")
        .await
        .expect("playbook registered")
        .expect("playbook should run");
    drop(active);

    assert_eq!(
        observer.states_for("apply-schema"),
        vec![
            PlaybookState::Registered,
            PlaybookState::Running,
            PlaybookState::Stopped,
            PlaybookState::Running,
            PlaybookState::Stopped
        ]
    );
    assert_eq!(observer.runs_for("apply-schema"), vec![0, 1, 1, 2, 2]);
}

#[tokio::test]
async fn attach_reopened_arena_starts_its_playbooks_from_zero() {
    let observer = RecordingPlaybookObserver::new();
    let (mut a_match, bus) = attached_match(
        Match::new("healthy", vec![], vec![])
            .register_playbook(probe_playbook("apply-schema").into_playbook(), true),
        observer.clone(),
    );
    let publisher = arena::lifecycle::ArenaLifecyclePublisher::new("arena-under-test", vec![]);
    a_match.start(&publisher).await.expect("match should open");
    assert_eq!(observer.runs_for("apply-schema").last(), Some(&1));

    a_match.attach_playbook_publisher(
        ARENA_ID,
        Arc::clone(&bus) as Arc<dyn Publisher<PlaybookEvent>>,
    );

    assert_eq!(observer.runs_for("apply-schema").last(), Some(&0));
    assert_eq!(
        observer.states_for("apply-schema").last(),
        Some(&PlaybookState::Registered)
    );
}

