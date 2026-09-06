#[path = "subjects.rs"]
mod subjects;

use arena::lifecycle::{ArenaLifecycleState, RunnableState};
use arena::matches::Match;
use arena::{ClosedArena, MatchTrait};
use std::sync::Arc;
use subjects::{
    probe_component, probe_dependency, probe_playbook, Behaviour, EventScopeRecordingLayer,
    RecordedEvents, StateRecorder,
};
use tracing_subscriber::layer::SubscriberExt;

fn arena_with(matches: Vec<Box<dyn MatchTrait>>) -> (ClosedArena, Arc<StateRecorder>) {
    let recorder = Arc::new(StateRecorder::default());
    let closed = ClosedArena::new("test-arena".to_string(), matches).observe(recorder.clone());
    (closed, recorder)
}

#[tokio::test]
async fn open_healthy_arena_returns_open_arena_in_arena_open() {
    let a_match = Match::new(
        "healthy",
        vec![probe_dependency("postgres-1").into_dependency()],
        vec![probe_component("api").into_component()],
    );
    let (closed, recorder) = arena_with(vec![Box::new(a_match)]);

    let open = closed.open().await.expect("arena should open");

    assert_eq!(open.state().state, ArenaLifecycleState::ArenaOpen);
    assert_eq!(recorder.states().last(), Some(&ArenaLifecycleState::ArenaOpen));
}

#[tokio::test]
async fn open_happy_path_emits_states_in_order() {
    let a_match = Match::new(
        "healthy",
        vec![probe_dependency("postgres-1").into_dependency()],
        vec![probe_component("api").into_component()],
    );
    let (closed, recorder) = arena_with(vec![Box::new(a_match)]);

    let _open = closed.open().await.expect("arena should open");

    assert_eq!(
        recorder.states(),
        vec![
            ArenaLifecycleState::ArenaStarting,
            ArenaLifecycleState::DependenciesStarting,
            ArenaLifecycleState::DependenciesStarted,
            ArenaLifecycleState::PlaybooksRunning,
            ArenaLifecycleState::PlaybooksComplete,
            ArenaLifecycleState::ComponentsStarting,
            ArenaLifecycleState::ComponentsStarted,
            ArenaLifecycleState::ArenaOpen,
        ]
    );
}

#[tokio::test]
async fn open_dependency_readiness_failure_returns_faulted_arena_state() {
    let a_match = Match::new(
        "faulting",
        vec![probe_dependency("postgres-1")
            .behaving(Behaviour::FailStart)
            .into_dependency()],
        vec![probe_component("api").into_component()],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let state = closed.open().await.expect_err("arena should fault");

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert_eq!(state.faults.len(), 1);
    assert_eq!(state.faults[0].id, "postgres-1");
    assert!(state.faults[0].message.contains("readiness check"));
}

#[tokio::test]
async fn open_dependency_fault_is_owned_by_that_dependency_state() {
    let a_match = Match::new(
        "faulting",
        vec![
            probe_dependency("postgres-1")
                .behaving(Behaviour::FailStart)
                .into_dependency(),
            probe_dependency("kafka-1").into_dependency(),
        ],
        vec![],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let state = closed.open().await.expect_err("arena should fault");

    let faulting = state.dependency("postgres-1").expect("postgres-1 in state");
    assert_eq!(faulting.faults.len(), 1);
    let healthy = state.dependency("kafka-1").expect("kafka-1 in state");
    assert!(healthy.faults.is_empty());
    assert_eq!(healthy.state, RunnableState::Stopped);
}

#[tokio::test]
async fn open_two_dependencies_fail_aggregates_both_faults() {
    let a_match = Match::new(
        "faulting",
        vec![
            probe_dependency("postgres-1")
                .behaving(Behaviour::FailStart)
                .into_dependency(),
            probe_dependency("kafka-1")
                .behaving(Behaviour::FailStart)
                .into_dependency(),
        ],
        vec![],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let state = closed.open().await.expect_err("arena should fault");

    let mut ids: Vec<&str> = state.faults.iter().map(|f| f.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, vec!["kafka-1", "postgres-1"]);
}

#[tokio::test]
async fn open_component_fault_stops_every_started_dependency() {
    let dependency = probe_dependency("postgres-1");
    let dependency_counts = dependency.counts();
    let a_match = Match::new(
        "faulting",
        vec![dependency.into_dependency()],
        vec![probe_component("api")
            .behaving(Behaviour::FailStart)
            .into_component()],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let state = closed.open().await.expect_err("arena should fault");

    assert_eq!(
        state.dependency("postgres-1").map(|d| d.state),
        Some(RunnableState::Stopped)
    );
    assert_eq!(dependency_counts.stops(), 1);
    assert_eq!(dependency_counts.force_stops(), 1);
}

#[tokio::test]
async fn open_fault_calls_force_stop_on_every_subject_exactly_once() {
    let dependency = probe_dependency("postgres-1");
    let dependency_counts = dependency.counts();
    let component = probe_component("api").behaving(Behaviour::FailStart);
    let component_counts = component.counts();
    let a_match = Match::new(
        "faulting",
        vec![dependency.into_dependency()],
        vec![component.into_component()],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let _state = closed.open().await.expect_err("arena should fault");

    assert_eq!(dependency_counts.force_stops(), 1);
    assert_eq!(component_counts.force_stops(), 1);
}

#[tokio::test]
async fn open_subject_reporting_stopped_still_receives_force_stop() {
    let dependency = probe_dependency("liar").behaving(Behaviour::ReportStoppedWithoutStopping);
    let counts = dependency.counts();
    let a_match = Match::new(
        "faulting",
        vec![
            dependency.into_dependency(),
            probe_dependency("postgres-1")
                .behaving(Behaviour::FailStart)
                .into_dependency(),
        ],
        vec![],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let _state = closed.open().await.expect_err("arena should fault");

    assert_eq!(counts.stops(), 0, "graceful stop skips a subject reporting stopped");
    assert_eq!(counts.force_stops(), 1, "forced teardown must not trust the reported state");
}

#[tokio::test]
async fn open_subject_ignoring_force_stop_ends_faulted_with_recorded_fault() {
    let a_match = Match::new(
        "faulting",
        vec![
            probe_dependency("stuck")
                .behaving(Behaviour::ResistTeardown)
                .into_dependency(),
            probe_dependency("postgres-1")
                .behaving(Behaviour::FailStart)
                .into_dependency(),
        ],
        vec![],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let state = closed.open().await.expect_err("arena should fault");

    let stuck = state.dependency("stuck").expect("stuck in state");
    assert_eq!(stuck.state, RunnableState::Faulted);
    assert!(!stuck.faults.is_empty(), "a faulted subject must explain itself");
    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
}

#[tokio::test]
async fn open_subject_stop_panics_still_reaches_force_stop() {
    let dependency = probe_dependency("panicky").behaving(Behaviour::PanicStop);
    let counts = dependency.counts();
    let a_match = Match::new(
        "faulting",
        vec![
            dependency.into_dependency(),
            probe_dependency("postgres-1")
                .behaving(Behaviour::FailStart)
                .into_dependency(),
        ],
        vec![],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let state = closed.open().await.expect_err("arena should fault");

    assert_eq!(counts.stops(), 1);
    assert_eq!(counts.force_stops(), 1);
    assert_eq!(
        state.dependency("panicky").map(|d| d.state),
        Some(RunnableState::Stopped)
    );
}

#[tokio::test]
async fn open_panicking_dependency_returns_fault_and_still_tears_down() {
    let panicky = probe_dependency("panicky").behaving(Behaviour::PanicStart);
    let panicky_counts = panicky.counts();
    let healthy = probe_dependency("postgres-1");
    let healthy_counts = healthy.counts();
    let a_match = Match::new(
        "faulting",
        vec![healthy.into_dependency(), panicky.into_dependency()],
        vec![],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let state = closed.open().await.expect_err("arena should fault");

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert!(state
        .faults
        .iter()
        .any(|f| f.id == "panicky"
            && f.message == "failed to start"
            && f.faults.iter().any(|c| c.message.contains("start failed"))));
    assert_eq!(panicky_counts.force_stops(), 1);
    assert_eq!(healthy_counts.force_stops(), 1);
    assert_eq!(
        state.dependency("postgres-1").map(|d| d.state),
        Some(RunnableState::Stopped)
    );
}

#[tokio::test]
async fn open_panicking_playbook_returns_fault_and_tears_down() {
    let a_match = Match::new(
        "faulting",
        vec![probe_dependency("postgres-1").into_dependency()],
        vec![],
    )
    .register_playbook(
        probe_playbook("seed")
            .behaving(Behaviour::PanicStart)
            .into_playbook(),
        true,
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let state = closed.open().await.expect_err("arena should fault");

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert!(state
        .faults
        .iter()
        .any(|f| f.id == "seed"
            && f.message == "failed to run"
            && f.faults.iter().any(|c| c.message.contains("run failed"))));
    assert_eq!(
        state.dependency("postgres-1").map(|d| d.state),
        Some(RunnableState::Stopped)
    );
}

#[tokio::test]
async fn open_failing_playbook_returns_fault_and_tears_down() {
    let a_match = Match::new(
        "faulting",
        vec![probe_dependency("postgres-1").into_dependency()],
        vec![],
    )
    .register_playbook(
        probe_playbook("seed")
            .behaving(Behaviour::FailStart)
            .into_playbook(),
        true,
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let state = closed.open().await.expect_err("arena should fault");

    assert!(state
        .faults
        .iter()
        .any(|f| f.id == "seed" && f.message.contains("seed data rejected")));
}

#[tokio::test]
async fn open_fault_emits_arena_faulted_only_after_arena_teardown() {
    let a_match = Match::new(
        "faulting",
        vec![probe_dependency("postgres-1")
            .behaving(Behaviour::FailStart)
            .into_dependency()],
        vec![],
    );
    let (closed, recorder) = arena_with(vec![Box::new(a_match)]);

    let _state = closed.open().await.expect_err("arena should fault");

    let states = recorder.states();
    let teardown = states
        .iter()
        .position(|s| *s == ArenaLifecycleState::ArenaTeardown)
        .expect("arena teardown must be emitted");
    let faulted = states
        .iter()
        .position(|s| *s == ArenaLifecycleState::ArenaFaulted)
        .expect("arena faulted must be emitted");

    assert!(teardown < faulted, "teardown must complete before the arena reports faulted: {states:?}");
    assert_eq!(states.last(), Some(&ArenaLifecycleState::ArenaFaulted));
}

#[tokio::test]
async fn open_later_match_faults_stops_earlier_match() {
    let healthy_dependency = probe_dependency("postgres-1");
    let healthy_counts = healthy_dependency.counts();
    let healthy = Match::new("healthy", vec![healthy_dependency.into_dependency()], vec![]);
    let faulting = Match::new(
        "faulting",
        vec![probe_dependency("kafka-1")
            .behaving(Behaviour::FailStart)
            .into_dependency()],
        vec![],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(healthy), Box::new(faulting)]);

    let state = closed.open().await.expect_err("arena should fault");

    assert_eq!(
        state.dependency("postgres-1").map(|d| d.state),
        Some(RunnableState::Stopped)
    );
    assert_eq!(healthy_counts.force_stops(), 1);
}

#[tokio::test]
async fn close_happy_path_runs_arena_teardown_before_arena_closed() {
    let dependency = probe_dependency("postgres-1");
    let counts = dependency.counts();
    let a_match = Match::new("healthy", vec![dependency.into_dependency()], vec![]);
    let (closed, recorder) = arena_with(vec![Box::new(a_match)]);

    let open = closed.open().await.expect("arena should open");
    let _closed = open.close().await.expect("arena should close");

    let states = recorder.states();
    let teardown = states
        .iter()
        .position(|s| *s == ArenaLifecycleState::ArenaTeardown)
        .expect("arena teardown must be emitted");
    let arena_closed = states
        .iter()
        .position(|s| *s == ArenaLifecycleState::ArenaClosed)
        .expect("arena closed must be emitted");

    assert!(teardown < arena_closed, "{states:?}");
    assert_eq!(counts.stops(), 1);
    assert_eq!(counts.force_stops(), 1);
}

#[tokio::test]
async fn close_faulted_component_ends_arena_faulted_not_arena_closed() {
    let a_match = Match::new(
        "healthy",
        vec![],
        vec![probe_component("api")
            .behaving(Behaviour::ResistTeardown)
            .into_component()],
    );
    let (closed, recorder) = arena_with(vec![Box::new(a_match)]);

    let open = closed.open().await.expect("arena should open");
    let state = open.close().await.expect_err("close should report the fault");

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert_eq!(
        state.component("api").map(|c| c.state),
        Some(RunnableState::Faulted)
    );
    assert!(!recorder
        .states()
        .contains(&ArenaLifecycleState::ArenaClosed));
}

#[tokio::test]
async fn close_stop_fault_returns_faulted_state_without_panicking() {
    let a_match = Match::new(
        "healthy",
        vec![probe_dependency("postgres-1")
            .behaving(Behaviour::FailStop)
            .into_dependency()],
        vec![],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let open = closed.open().await.expect("arena should open");
    let state = open.close().await.expect_err("close should report the fault");

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert!(state.faults.iter().any(|f| f.id == "postgres-1"));
    assert_eq!(
        state.dependency("postgres-1").map(|d| d.state),
        Some(RunnableState::Stopped),
        "the forced sweep must recover a dependency whose graceful stop failed"
    );
}

#[tokio::test]
async fn close_called_twice_stops_subjects_once() {
    let dependency = probe_dependency("postgres-1");
    let counts = dependency.counts();
    let a_match = Match::new("healthy", vec![dependency.into_dependency()], vec![]);
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let open = closed.open().await.expect("arena should open");
    let reclosed = open.close().await.expect("arena should close");
    drop(reclosed);

    assert_eq!(counts.stops(), 1);
    assert_eq!(counts.force_stops(), 1);
}

#[test]
fn drop_open_arena_releases_every_subject() {
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let dependency = probe_dependency("postgres-1");
        let counts = dependency.counts();
        let a_match = Match::new("healthy", vec![dependency.into_dependency()], vec![]);
        let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

        let open = closed.open().await.expect("arena should open");
        drop(open);

        assert_eq!(counts.releases(), 1);
        assert_eq!(
            counts.stops(),
            0,
            "drop must not await a graceful stop; close() is the graceful path"
        );
        assert_eq!(counts.force_stops(), 0, "drop must not await a forced stop");
    });
}

#[test]
fn drop_open_arena_inside_current_thread_runtime_does_not_hang() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("current thread runtime");
    runtime.block_on(async {
        let dependency = probe_dependency("postgres-1");
        let counts = dependency.counts();
        let a_match = Match::new("healthy", vec![dependency.into_dependency()], vec![]);
        let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

        let open = closed.open().await.expect("arena should open");
        drop(open);

        assert_eq!(counts.releases(), 1);
    });
}

#[test]
fn drop_open_arena_stop_panic_does_not_abort() {
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(async {
        let dependency = probe_dependency("panicky").behaving(Behaviour::PanicStop);
        let counts = dependency.counts();
        let a_match = Match::new("healthy", vec![dependency.into_dependency()], vec![]);
        let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

        let open = closed.open().await.expect("arena should open");
        drop(open);

        assert_eq!(counts.releases(), 1);
    });
}

#[tokio::test]
async fn state_closed_arena_before_open_returns_arena_created() {
    let a_match = Match::new(
        "healthy",
        vec![probe_dependency("postgres-1").into_dependency()],
        vec![],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let state = closed.state();

    assert_eq!(state.state, ArenaLifecycleState::ArenaCreated);
    assert_eq!(
        state.dependency("postgres-1").map(|d| d.state),
        Some(RunnableState::NotStarted)
    );
}

#[tokio::test]
async fn dependency_found_in_later_match_returns_dependency() {
    let empty = Match::new("empty", vec![], vec![]);
    let real = Match::new(
        "real-match",
        vec![probe_dependency("dep-1").into_dependency()],
        vec![],
    )
    .register_playbook(probe_playbook("pb-1").into_playbook(), false);
    let (closed, _recorder) = arena_with(vec![Box::new(empty), Box::new(real)]);

    let mut open = closed.open().await.expect("arena should open");

    assert_eq!(open.dependency("dep-1").map(|d| d.identifier()), Some("dep-1"));
    assert!(open.dependency("missing").is_none());
    assert_eq!(
        open.dependency_mut("dep-1").map(|d| d.identifier()),
        Some("dep-1")
    );

    let active = open.run_playbook("pb-1").await.expect("playbook registered");
    assert_eq!(active.expect("playbook should run").identifier(), "pb-1");
    assert!(open.run_playbook("missing").await.is_none());

    let _closed = open.close().await.expect("arena should close");
}

struct BareMatch {
    start_faults: Vec<arena::lifecycle::Fault>,
    panic_on_start: bool,
    panic_on_force_stop: bool,
}

fn bare_match() -> BareMatch {
    BareMatch {
        start_faults: Vec::new(),
        panic_on_start: false,
        panic_on_force_stop: false,
    }
}

#[async_trait::async_trait]
impl MatchTrait for BareMatch {
    async fn start(
        &mut self,
        _ctx: &arena::lifecycle::LifecycleContext,
    ) -> Result<(), Vec<arena::lifecycle::Fault>> {
        if self.panic_on_start {
            panic!("match start failed");
        }
        if self.start_faults.is_empty() {
            Ok(())
        } else {
            Err(self.start_faults.clone())
        }
    }

    async fn stop(
        &mut self,
        _ctx: &arena::lifecycle::LifecycleContext,
    ) -> Result<(), Vec<arena::lifecycle::Fault>> {
        Ok(())
    }

    async fn force_stop_all(&mut self) {
        if self.panic_on_force_stop {
            panic!("match forced teardown failed");
        }
    }
}

#[tokio::test]
async fn open_match_without_state_overrides_uses_empty_defaults() {
    let (closed, recorder) = arena_with(vec![Box::new(bare_match())]);

    let open = closed.open().await.expect("arena should open");

    let state = open.state();
    assert!(state.dependencies.is_empty());
    assert!(state.components.is_empty());
    assert!(open.dependency("anything").is_none());
    assert!(open.run_playbook("anything").await.is_none());
    assert_eq!(recorder.states().last(), Some(&ArenaLifecycleState::ArenaOpen));
}

#[tokio::test]
async fn open_match_start_panics_returns_arena_fault() {
    let mut faulting = bare_match();
    faulting.panic_on_start = true;
    let (closed, _recorder) = arena_with(vec![Box::new(faulting)]);

    let state = closed.open().await.expect_err("arena should fault");

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert_eq!(state.faults.len(), 1);
    assert_eq!(state.faults[0].subject, arena::lifecycle::Subject::Arena);
    assert_eq!(state.faults[0].message, "failed to start");
    assert!(state.faults[0]
        .faults
        .iter()
        .any(|c| c.message.contains("start failed")));
}

#[tokio::test]
async fn open_match_force_stop_panics_records_arena_fault() {
    let mut faulting = bare_match();
    faulting.start_faults = vec![arena::lifecycle::Fault::dependency("dep-1", "boom")];
    faulting.panic_on_force_stop = true;
    let (closed, _recorder) = arena_with(vec![Box::new(faulting)]);

    let state = closed.open().await.expect_err("arena should fault");

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert!(state
        .faults
        .iter()
        .any(|f| f.message == "failed to stop"
            && f.faults.iter().any(|c| c.message.contains("match forced teardown failed"))));
}

#[tokio::test]
async fn close_match_stop_faults_are_recorded_on_the_arena() {
    struct StopFaultingMatch;

    #[async_trait::async_trait]
    impl MatchTrait for StopFaultingMatch {
        async fn start(
            &mut self,
            _ctx: &arena::lifecycle::LifecycleContext,
        ) -> Result<(), Vec<arena::lifecycle::Fault>> {
            Ok(())
        }

        async fn stop(
            &mut self,
            _ctx: &arena::lifecycle::LifecycleContext,
        ) -> Result<(), Vec<arena::lifecycle::Fault>> {
            Err(vec![arena::lifecycle::Fault::dependency(
                "dep-1",
                "stop did not complete",
            )])
        }
    }

    let (closed, _recorder) = arena_with(vec![Box::new(StopFaultingMatch)]);

    let open = closed.open().await.expect("arena should open");
    let state = open.close().await.expect_err("close should report the fault");

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert!(state.faults.iter().any(|f| f.id == "dep-1"));
}

#[tokio::test]
async fn close_match_stop_panics_is_recorded_on_the_arena() {
    struct PanickingStopMatch;

    #[async_trait::async_trait]
    impl MatchTrait for PanickingStopMatch {
        async fn start(
            &mut self,
            _ctx: &arena::lifecycle::LifecycleContext,
        ) -> Result<(), Vec<arena::lifecycle::Fault>> {
            Ok(())
        }

        async fn stop(
            &mut self,
            _ctx: &arena::lifecycle::LifecycleContext,
        ) -> Result<(), Vec<arena::lifecycle::Fault>> {
            panic!("match stop failed");
        }
    }

    let (closed, _recorder) = arena_with(vec![Box::new(PanickingStopMatch)]);

    let open = closed.open().await.expect("arena should open");
    let state = open.close().await.expect_err("close should report the fault");

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert!(state
        .faults
        .iter()
        .any(|f| f.message == "failed to stop"
            && f.faults.iter().any(|c| c.message.contains("stop failed"))));
}

#[tokio::test]
async fn open_silently_faulted_subject_gets_an_arena_raised_fault() {
    let a_match = Match::new(
        "faulting",
        vec![
            probe_dependency("silent")
                .behaving(Behaviour::FaultSilently)
                .into_dependency(),
            probe_dependency("postgres-1")
                .behaving(Behaviour::FailStart)
                .into_dependency(),
        ],
        vec![probe_component("silent-component")
            .behaving(Behaviour::FaultSilently)
            .into_component()],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let state = closed.open().await.expect_err("arena should fault");

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert_eq!(
        state.dependency("silent").map(|d| d.state),
        Some(RunnableState::Faulted)
    );
    assert!(
        state.faults.iter().any(|f| f.message.contains("dependency 'silent' is faulted")),
        "the arena must explain a subject that faulted without recording a reason: {:?}",
        state.faults
    );
    assert!(state
        .faults
        .iter()
        .any(|f| f.message.contains("component 'silent-component' is faulted")));
}

#[tokio::test]
async fn state_open_arena_reflects_recorded_faults() {
    let a_match = Match::new(
        "healthy",
        vec![probe_dependency("postgres-1").into_dependency()],
        vec![],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let open = closed.open().await.expect("arena should open");

    let state = open.state();
    assert_eq!(state.state, ArenaLifecycleState::ArenaOpen);
    assert_eq!(
        state.dependency("postgres-1").map(|d| d.state),
        Some(RunnableState::Started)
    );
    assert!(state.faults.is_empty());
    assert!(format!("{open:?}").contains("OpenArena"));
}

#[tokio::test]
async fn open_observer_panicking_on_teardown_still_force_stops_every_subject() {
    let healthy = probe_dependency("postgres-1");
    let healthy_counts = healthy.counts();
    let a_match = Match::new(
        "faulting",
        vec![
            healthy.into_dependency(),
            probe_dependency("kafka-1")
                .behaving(Behaviour::FailStart)
                .into_dependency(),
        ],
        vec![],
    );
    let observer = Arc::new(subjects::PanickingObserver::panicking_on(
        ArenaLifecycleState::ArenaTeardown,
    ));
    let closed = ClosedArena::new("test-arena".to_string(), vec![Box::new(a_match)])
        .observe(observer.clone());

    let state = closed.open().await.expect_err("arena should fault");

    assert_eq!(
        healthy_counts.force_stops(),
        1,
        "a panicking observer must not cancel the forced sweep"
    );
    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert!(observer.seen().contains(&ArenaLifecycleState::ArenaTeardown));
}

#[tokio::test]
async fn close_observer_panicking_on_closing_still_force_stops_every_subject() {
    let dependency = probe_dependency("postgres-1");
    let counts = dependency.counts();
    let a_match = Match::new("healthy", vec![dependency.into_dependency()], vec![]);
    let observer = Arc::new(subjects::PanickingObserver::panicking_on(
        ArenaLifecycleState::ArenaClosing,
    ));
    let closed = ClosedArena::new("test-arena".to_string(), vec![Box::new(a_match)])
        .observe(observer.clone());

    let open = closed.open().await.expect("arena should open");
    let _closed = open.close().await.expect("arena should close");

    assert_eq!(counts.stops(), 1);
    assert_eq!(
        counts.force_stops(),
        1,
        "a panicking observer must not cancel teardown"
    );
}

#[tokio::test]
async fn close_dependency_stop_panic_is_reported_as_a_fault() {
    let a_match = Match::new(
        "healthy",
        vec![probe_dependency("panicky")
            .behaving(Behaviour::PanicStop)
            .into_dependency()],
        vec![],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let open = closed.open().await.expect("arena should open");
    let state = open.close().await.expect_err("close should report the panic");

    assert_eq!(state.state, ArenaLifecycleState::ArenaFaulted);
    assert!(
        state
            .faults
            .iter()
            .any(|f| f.id == "panicky"
                && f.message == "failed to stop"
                && f.faults.iter().any(|c| c.message.contains("stop failed"))),
        "a panic escaping stop must surface as a fault with the panic text as its cause: {:?}",
        state.faults
    );
}

#[tokio::test]
async fn close_component_stop_panic_is_reported_as_a_fault() {
    let a_match = Match::new(
        "healthy",
        vec![],
        vec![probe_component("api")
            .behaving(Behaviour::PanicStop)
            .into_component()],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let open = closed.open().await.expect("arena should open");
    let state = open.close().await.expect_err("close should report the panic");

    assert!(state
        .faults
        .iter()
        .any(|f| f.id == "api"
            && f.message == "failed to stop"
            && f.faults.iter().any(|c| c.message.contains("stop failed"))));
}

#[tokio::test]
async fn close_dependency_stop_fault_is_reported_by_the_arena() {
    let a_match = Match::new(
        "healthy",
        vec![probe_dependency("postgres-1")
            .behaving(Behaviour::FailStop)
            .into_dependency()],
        vec![],
    );
    let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);

    let open = closed.open().await.expect("arena should open");
    let state = open.close().await.expect_err("close should report the fault");

    assert!(state
        .faults
        .iter()
        .any(|f| f.id == "postgres-1" && f.message.contains("stop did not complete")));
}

fn events_recorded_during_open_and_close() -> Arc<RecordedEvents> {
    let recorded = Arc::new(RecordedEvents::default());
    let subscriber =
        tracing_subscriber::registry().with(EventScopeRecordingLayer(Arc::clone(&recorded)));

    tracing::subscriber::with_default(subscriber, || {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
            let a_match = Match::new(
                "spans",
                vec![probe_dependency("postgres-1").into_dependency()],
                vec![probe_component("api").into_component()],
            );
            let (closed, _recorder) = arena_with(vec![Box::new(a_match)]);
            let open = closed.open().await.expect("arena should open");
            open.close().await.expect("arena should close");
        });
    });

    recorded
}

#[test]
fn open_healthy_arena_records_dependency_start_inside_its_subject_span() {
    let recorded = events_recorded_during_open_and_close();

    let started = recorded.with_message("dependency started");
    assert!(!started.is_empty(), "no dependency start record was emitted");
    for event in started {
        assert_eq!(event.arena(), Some("test-arena"), "scope {:?}", event.scope);
        assert_eq!(
            event.subject(),
            Some("dependency.postgres-1"),
            "scope {:?}",
            event.scope
        );
    }
}

#[test]
fn open_healthy_arena_records_component_start_inside_its_subject_span() {
    let recorded = events_recorded_during_open_and_close();

    let started = recorded.with_message("component started");
    assert!(!started.is_empty(), "no component start record was emitted");
    for event in started {
        assert_eq!(event.arena(), Some("test-arena"), "scope {:?}", event.scope);
        assert_eq!(
            event.subject(),
            Some("component.api"),
            "scope {:?}",
            event.scope
        );
    }
}

#[test]
fn open_healthy_arena_records_arena_records_under_the_arena_span_only() {
    let recorded = events_recorded_during_open_and_close();

    let opening = recorded.with_message("opening");
    assert!(!opening.is_empty(), "no arena open record was emitted");
    for event in opening {
        assert_eq!(event.arena(), Some("test-arena"), "scope {:?}", event.scope);
        assert_eq!(event.subject(), None, "scope {:?}", event.scope);
    }
}
