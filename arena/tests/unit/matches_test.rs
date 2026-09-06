#[path = "subjects.rs"]
mod subjects;

use arena::lifecycle::{ArenaLifecycleState, LifecycleContext, RunnableState};
use arena::matches::{Match, MatchTrait};
use std::sync::{Arc, Mutex};
use subjects::{probe_component, probe_dependency, probe_playbook, Behaviour, StateRecorder};

fn context() -> (LifecycleContext, Arc<StateRecorder>) {
    let recorder = Arc::new(StateRecorder::default());
    let context = LifecycleContext::new("test-arena", vec![recorder.clone()]);
    (context, recorder)
}

#[tokio::test]
async fn start_healthy_match_starts_dependencies_then_components() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let mut a_match = Match::new(
        "healthy",
        vec![probe_dependency("postgres-1")
            .recording_order(order.clone())
            .into_dependency()],
        vec![probe_component("api")
            .recording_order(order.clone())
            .into_component()],
    );
    let (ctx, _recorder) = context();

    a_match.start(&ctx).await.expect("match should start");

    assert_eq!(
        order.lock().unwrap().clone(),
        vec!["postgres-1:start", "api:start"]
    );
}

#[tokio::test]
async fn start_healthy_match_emits_every_start_state() {
    let mut a_match = Match::new(
        "healthy",
        vec![probe_dependency("postgres-1").into_dependency()],
        vec![probe_component("api").into_component()],
    );
    let (ctx, recorder) = context();

    a_match.start(&ctx).await.expect("match should start");

    assert_eq!(
        recorder.states(),
        vec![
            ArenaLifecycleState::DependenciesStarting,
            ArenaLifecycleState::DependenciesStarted,
            ArenaLifecycleState::PlaybooksRunning,
            ArenaLifecycleState::PlaybooksComplete,
            ArenaLifecycleState::ComponentsStarting,
            ArenaLifecycleState::ComponentsStarted,
        ]
    );
}

#[tokio::test]
async fn start_called_twice_starts_subjects_once() {
    let dependency = probe_dependency("postgres-1");
    let counts = dependency.counts();
    let mut a_match = Match::new("healthy", vec![dependency.into_dependency()], vec![]);
    let (ctx, _recorder) = context();

    a_match.start(&ctx).await.expect("match should start");
    a_match.start(&ctx).await.expect("second start is a no-op");

    assert_eq!(counts.starts(), 1);
}

#[tokio::test]
async fn start_dependency_fault_returns_fault_and_skips_components() {
    let component = probe_component("api");
    let component_counts = component.counts();
    let mut a_match = Match::new(
        "faulting",
        vec![probe_dependency("postgres-1")
            .behaving(Behaviour::FailStart)
            .into_dependency()],
        vec![component.into_component()],
    );
    let (ctx, _recorder) = context();

    let faults = a_match.start(&ctx).await.expect_err("match should fault");

    assert_eq!(faults.len(), 1);
    assert_eq!(faults[0].id, "postgres-1");
    assert_eq!(component_counts.starts(), 0);
}

#[tokio::test]
async fn start_dependency_fault_emits_stopping_states() {
    let mut a_match = Match::new(
        "faulting",
        vec![probe_dependency("postgres-1")
            .behaving(Behaviour::FailStart)
            .into_dependency()],
        vec![],
    );
    let (ctx, recorder) = context();

    let _faults = a_match.start(&ctx).await.expect_err("match should fault");

    let states = recorder.states();
    assert!(states.contains(&ArenaLifecycleState::ComponentsStopping));
    assert!(states.contains(&ArenaLifecycleState::DependenciesStopped));
    assert!(!states.contains(&ArenaLifecycleState::DependenciesStarted));
}

#[tokio::test]
async fn start_later_dependency_fault_stops_earlier_started_dependencies() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let healthy = probe_dependency("postgres-1").recording_order(order.clone());
    let counts = healthy.counts();
    let mut a_match = Match::new(
        "faulting",
        vec![
            healthy.into_dependency(),
            probe_dependency("kafka-1")
                .behaving(Behaviour::FailStart)
                .into_dependency(),
        ],
        vec![],
    );
    let (ctx, _recorder) = context();

    let _faults = a_match.start(&ctx).await.expect_err("match should fault");

    assert_eq!(counts.stops(), 1);
    assert!(order.lock().unwrap().contains(&"postgres-1:stop".to_string()));
}

#[tokio::test]
async fn start_component_fault_returns_fault_and_stops_dependencies() {
    let dependency = probe_dependency("postgres-1");
    let counts = dependency.counts();
    let mut a_match = Match::new(
        "faulting",
        vec![dependency.into_dependency()],
        vec![probe_component("api")
            .behaving(Behaviour::FailStart)
            .into_component()],
    );
    let (ctx, _recorder) = context();

    let faults = a_match.start(&ctx).await.expect_err("match should fault");

    assert_eq!(faults[0].id, "api");
    assert_eq!(counts.stops(), 1);
}

#[tokio::test]
async fn start_playbook_fault_returns_fault_and_stops_dependencies() {
    let dependency = probe_dependency("postgres-1");
    let counts = dependency.counts();
    let mut a_match = Match::new("faulting", vec![dependency.into_dependency()], vec![])
        .register_playbook(
            probe_playbook("seed")
                .behaving(Behaviour::FailStart)
                .into_playbook(),
            true,
        );
    let (ctx, _recorder) = context();

    let faults = a_match.start(&ctx).await.expect_err("match should fault");

    assert_eq!(faults[0].id, "seed");
    assert_eq!(counts.stops(), 1);
}

#[tokio::test]
async fn register_playbook_exec_on_start_false_skips_run() {
    let dependency = probe_dependency("postgres-1");
    let mut a_match = Match::new("healthy", vec![dependency.into_dependency()], vec![])
        .register_playbook(
            probe_playbook("seed")
                .behaving(Behaviour::FailStart)
                .into_playbook(),
            false,
        );
    let (ctx, _recorder) = context();

    a_match
        .start(&ctx)
        .await
        .expect("a playbook not marked for startup must not run");
}

#[tokio::test]
async fn run_playbook_known_id_returns_active() {
    let a_match = Match::new("healthy", vec![], vec![])
        .register_playbook(probe_playbook("seed").into_playbook(), false);

    let active = a_match.run_playbook("seed").await.expect("playbook registered");

    assert_eq!(active.expect("playbook should run").identifier(), "seed");
}

#[tokio::test]
async fn run_playbook_unknown_id_returns_none() {
    let a_match = Match::new("healthy", vec![], vec![])
        .register_playbook(probe_playbook("seed").into_playbook(), false);

    assert!(a_match.run_playbook("missing").await.is_none());
}

#[tokio::test]
async fn run_playbook_failing_playbook_returns_fault() {
    let a_match = Match::new("healthy", vec![], vec![]).register_playbook(
        probe_playbook("seed")
            .behaving(Behaviour::FailStart)
            .into_playbook(),
        false,
    );

    let outcome = a_match.run_playbook("seed").await.expect("playbook registered");

    assert!(outcome.is_err());
}

#[tokio::test]
async fn dependency_nested_two_levels_returns_grandchild() {
    let grandchild = probe_dependency("grandchild").into_dependency();
    let child = probe_dependency("child").with_child(grandchild).into_dependency();
    let parent = probe_dependency("parent").with_child(child).into_dependency();
    let a_match = Match::new("healthy", vec![parent], vec![]);

    assert_eq!(
        a_match.dependency("grandchild").map(|d| d.identifier()),
        Some("grandchild")
    );
    assert!(a_match.dependency("missing").is_none());
}

#[tokio::test]
async fn dependency_mut_nested_two_levels_returns_grandchild() {
    let grandchild = probe_dependency("grandchild").into_dependency();
    let child = probe_dependency("child").with_child(grandchild).into_dependency();
    let parent = probe_dependency("parent").with_child(child).into_dependency();
    let mut a_match = Match::new("healthy", vec![parent], vec![]);

    assert_eq!(
        a_match.dependency_mut("grandchild").map(|d| d.identifier()),
        Some("grandchild")
    );
    assert!(a_match.dependency_mut("missing").is_none());
}

#[tokio::test]
async fn dependency_states_nested_tree_mirrors_children() {
    let grandchild = probe_dependency("grandchild").into_dependency();
    let child = probe_dependency("child").with_child(grandchild).into_dependency();
    let parent = probe_dependency("parent").with_child(child).into_dependency();
    let a_match = Match::new("healthy", vec![parent], vec![]);

    let states = a_match.dependency_states();

    assert_eq!(states.len(), 1);
    assert_eq!(states[0].id, "parent");
    assert_eq!(states[0].children[0].id, "child");
    assert_eq!(states[0].children[0].children[0].id, "grandchild");
    assert_eq!(states[0].children[0].children[0].state, RunnableState::NotStarted);
}

#[tokio::test]
async fn component_states_nested_tree_mirrors_children() {
    let child = probe_component("worker").into_component();
    let parent = probe_component("api").with_child(child).into_component();
    let a_match = Match::new("healthy", vec![], vec![parent]);

    let states = a_match.component_states();

    assert_eq!(states[0].id, "api");
    assert_eq!(states[0].children[0].id, "worker");
}

#[tokio::test]
async fn force_stop_all_nested_children_reaches_every_level() {
    let grandchild = probe_dependency("grandchild");
    let grandchild_counts = grandchild.counts();
    let child = probe_dependency("child").with_child(grandchild.into_dependency());
    let child_counts = child.counts();
    let parent = probe_dependency("parent").with_child(child.into_dependency());
    let parent_counts = parent.counts();

    let nested_component = probe_component("worker");
    let nested_component_counts = nested_component.counts();
    let component = probe_component("api").with_child(nested_component.into_component());
    let component_counts = component.counts();

    let mut a_match = Match::new(
        "healthy",
        vec![parent.into_dependency()],
        vec![component.into_component()],
    );

    a_match.force_stop_all().await;

    assert_eq!(grandchild_counts.force_stops(), 1);
    assert_eq!(child_counts.force_stops(), 1);
    assert_eq!(parent_counts.force_stops(), 1);
    assert_eq!(nested_component_counts.force_stops(), 1);
    assert_eq!(component_counts.force_stops(), 1);
}

#[tokio::test]
async fn force_stop_all_called_twice_is_indistinguishable_from_once() {
    let dependency = probe_dependency("postgres-1");
    let counts = dependency.counts();
    let mut a_match = Match::new("healthy", vec![dependency.into_dependency()], vec![]);

    a_match.force_stop_all().await;
    let after_first = a_match.dependency_states();
    a_match.force_stop_all().await;
    let after_second = a_match.dependency_states();

    assert_eq!(counts.force_stops(), 2);
    assert_eq!(after_first[0].state, after_second[0].state);
    assert_eq!(after_first[0].faults, after_second[0].faults);
}

#[tokio::test]
async fn stop_without_start_stops_registered_component() {
    let component = probe_component("api");
    let counts = component.counts();
    let mut a_match = Match::new("healthy", vec![], vec![component.into_component()]);
    let (ctx, _recorder) = context();

    a_match.stop(&ctx).await.expect("stop should succeed");

    assert_eq!(counts.stops(), 0, "a component that never started needs no graceful stop");
}

#[tokio::test]
async fn stop_started_match_stops_components_before_dependencies() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let mut a_match = Match::new(
        "healthy",
        vec![probe_dependency("postgres-1")
            .recording_order(order.clone())
            .into_dependency()],
        vec![probe_component("api")
            .recording_order(order.clone())
            .into_component()],
    );
    let (ctx, _recorder) = context();

    a_match.start(&ctx).await.expect("match should start");
    a_match.stop(&ctx).await.expect("match should stop");

    let log = order.lock().unwrap().clone();
    let component_stop = log.iter().position(|e| e == "api:stop").expect("component stop");
    let dependency_stop = log
        .iter()
        .position(|e| e == "postgres-1:stop")
        .expect("dependency stop");
    assert!(component_stop < dependency_stop, "{log:?}");
}

#[tokio::test]
async fn force_stop_all_panicking_subject_still_reaches_the_rest() {
    let panicky = probe_dependency("panicky").behaving(Behaviour::PanicForceStop);
    let panicky_counts = panicky.counts();
    let healthy = probe_dependency("postgres-1");
    let healthy_counts = healthy.counts();
    let component = probe_component("api").behaving(Behaviour::PanicForceStop);
    let component_counts = component.counts();
    let mut a_match = Match::new(
        "faulting",
        vec![panicky.into_dependency(), healthy.into_dependency()],
        vec![component.into_component()],
    );

    a_match.force_stop_all().await;

    assert_eq!(component_counts.force_stops(), 1);
    assert_eq!(panicky_counts.force_stops(), 1);
    assert_eq!(
        healthy_counts.force_stops(),
        1,
        "a panicking forced teardown must not abandon the remaining subjects"
    );
}

#[tokio::test]
async fn stop_panicking_subject_still_stops_the_rest() {
    let panicky = probe_dependency("panicky").behaving(Behaviour::PanicStop);
    let panicky_counts = panicky.counts();
    let healthy = probe_dependency("postgres-1");
    let healthy_counts = healthy.counts();
    let component = probe_component("api").behaving(Behaviour::PanicStop);
    let component_counts = component.counts();
    let mut a_match = Match::new(
        "faulting",
        vec![panicky.into_dependency(), healthy.into_dependency()],
        vec![component.into_component()],
    );
    let (ctx, _recorder) = context();

    a_match.start(&ctx).await.expect("match should start");
    let faults = a_match
        .stop(&ctx)
        .await
        .expect_err("a panicking stop must be reported");

    assert_eq!(component_counts.stops(), 1);
    assert_eq!(panicky_counts.stops(), 1);
    assert_eq!(
        healthy_counts.stops(),
        1,
        "a panicking graceful stop must not abandon the remaining subjects"
    );
    let ids: Vec<&str> = faults.iter().map(|f| f.id.as_str()).collect();
    assert!(ids.contains(&"panicky"), "{ids:?}");
    assert!(ids.contains(&"api"), "{ids:?}");
    assert!(faults
        .iter()
        .all(|f| f.message.contains("panicked while stopping")));
}

#[tokio::test]
async fn stop_subject_reporting_inactive_is_not_gracefully_stopped() {
    let dependency = probe_dependency("liar").behaving(Behaviour::ReportStoppedWithoutStopping);
    let counts = dependency.counts();
    let component = probe_component("liar-component")
        .behaving(Behaviour::ReportStoppedWithoutStopping);
    let component_counts = component.counts();
    let mut a_match = Match::new(
        "healthy",
        vec![dependency.into_dependency()],
        vec![component.into_component()],
    );
    let (ctx, _recorder) = context();

    a_match.stop(&ctx).await.expect("stop should succeed");

    assert_eq!(counts.stops(), 0);
    assert_eq!(component_counts.stops(), 0);
}

#[tokio::test]
async fn start_component_child_starts_with_its_parent() {
    let child = probe_component("worker");
    let child_counts = child.counts();
    let parent = probe_component("api").with_child(child.into_component());
    let mut a_match = Match::new("healthy", vec![], vec![parent.into_component()]);
    let (ctx, _recorder) = context();

    a_match.start(&ctx).await.expect("match should start");

    assert_eq!(child_counts.starts(), 0, "the fake parent does not cascade start");
    assert_eq!(a_match.component_states()[0].children[0].id, "worker");
}

#[tokio::test]
async fn force_stop_all_nested_children_are_stopped_exactly_once() {
    let grandchild = probe_dependency("grandchild");
    let grandchild_counts = grandchild.counts();
    let child = probe_dependency("child").with_child(grandchild.into_dependency());
    let child_counts = child.counts();
    let parent = probe_dependency("parent").with_child(child.into_dependency());
    let parent_counts = parent.counts();

    let nested = probe_component("worker");
    let nested_counts = nested.counts();
    let component = probe_component("api").with_child(nested.into_component());
    let component_counts = component.counts();

    let mut a_match = Match::new(
        "healthy",
        vec![parent.into_dependency()],
        vec![component.into_component()],
    );

    a_match.force_stop_all().await;

    assert_eq!(parent_counts.force_stops(), 1);
    assert_eq!(child_counts.force_stops(), 1, "a subject tears down its own subtree");
    assert_eq!(grandchild_counts.force_stops(), 1);
    assert_eq!(component_counts.force_stops(), 1);
    assert_eq!(nested_counts.force_stops(), 1);
}

#[tokio::test]
async fn force_stop_all_repeated_on_resisting_subject_records_one_fault() {
    let dependency = probe_dependency("stuck").behaving(Behaviour::ResistTeardown);
    let counts = dependency.counts();
    let mut a_match = Match::new("faulting", vec![dependency.into_dependency()], vec![]);

    a_match.force_stop_all().await;
    a_match.force_stop_all().await;
    a_match.force_stop_all().await;

    let states = a_match.dependency_states();
    assert_eq!(counts.force_stops(), 3);
    assert_eq!(states[0].state, RunnableState::Faulted);
    assert_eq!(
        states[0].faults.len(),
        1,
        "a repeated forced teardown must not accumulate duplicate faults"
    );
}
