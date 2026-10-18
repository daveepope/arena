#[path = "subjects.rs"]
mod subjects;

use arena::lifecycle::{Fault, RunnableState};
use arena::playbook::lifecycle::PlaybookKind;
use arena::playbook::{ActivePlaybook, ManagedPlaybook, PlaybookType, UnmanagedPlaybook};
use std::sync::Arc;
use subjects::{observe_playbooks, probe_active_playbook, RecordingPlaybookObserver};

const ARENA_ID: &str = "arena-under-test";

fn observed(
    lifecycle: &Arc<arena::playbook::ScopedPlaybookLifecycle>,
) -> Arc<RecordingPlaybookObserver> {
    observe_playbooks(lifecycle, ARENA_ID, PlaybookKind::Scoped)
}

#[test]
fn new_managed_playbook_reports_itself_as_managed() {
    let playbook = ManagedPlaybook::new("apply-schema");

    assert_eq!(playbook.lifecycle().playbook_id(), "apply-schema");
    assert_eq!(playbook.lifecycle().playbook_type(), PlaybookType::Managed);
    assert_eq!(playbook.lifecycle().state(), RunnableState::NotStarted);
}

#[test]
fn new_unmanaged_playbook_reports_itself_as_unmanaged() {
    let playbook = UnmanagedPlaybook::new("seed-reference-data");

    assert_eq!(playbook.lifecycle().playbook_id(), "seed-reference-data");
    assert_eq!(playbook.lifecycle().playbook_type(), PlaybookType::Unmanaged);
}

#[tokio::test]
async fn apply_managed_playbook_stamps_every_event_as_managed() {
    let playbook = ManagedPlaybook::new("reset-orders");
    let observer = observed(playbook.lifecycle());

    let active = playbook
        .apply(async { Ok(probe_active_playbook(playbook.lifecycle())) })
        .await
        .expect("the playbook should run");
    drop(active);

    assert_eq!(
        observer.types_for("reset-orders"),
        vec![PlaybookType::Managed; 5]
    );
}

#[tokio::test]
async fn apply_unmanaged_playbook_stamps_every_event_as_unmanaged() {
    let playbook = UnmanagedPlaybook::new("seed-reference-data");
    let observer = observed(playbook.lifecycle());

    let active = playbook
        .apply(async { Ok(probe_active_playbook(playbook.lifecycle())) })
        .await
        .expect("the playbook should run");
    drop(active);

    assert_eq!(
        observer.types_for("seed-reference-data"),
        vec![PlaybookType::Unmanaged; 5]
    );
}

#[tokio::test]
async fn apply_managed_body_that_succeeds_reports_starting_then_started() {
    let playbook = ManagedPlaybook::new("reset-orders");
    let observer = observed(playbook.lifecycle());

    let outcome = playbook
        .apply(async { Ok(probe_active_playbook(playbook.lifecycle())) })
        .await;

    assert!(outcome.is_ok());
    assert_eq!(playbook.lifecycle().state(), RunnableState::Started);
    assert_eq!(
        observer.states(),
        vec![
            RunnableState::NotStarted,
            RunnableState::Starting,
            RunnableState::Started
        ]
    );
}

#[tokio::test]
async fn apply_unmanaged_body_that_succeeds_reports_starting_then_started() {
    let playbook = UnmanagedPlaybook::new("reset-orders");
    let observer = observed(playbook.lifecycle());

    let outcome = playbook
        .apply(async { Ok(probe_active_playbook(playbook.lifecycle())) })
        .await;

    assert!(outcome.is_ok());
    assert_eq!(
        observer.states(),
        vec![
            RunnableState::NotStarted,
            RunnableState::Starting,
            RunnableState::Started
        ]
    );
}

#[tokio::test]
async fn apply_body_that_faults_stops_at_starting_and_returns_the_fault() {
    let playbook = ManagedPlaybook::new("reset-orders");
    let observer = observed(playbook.lifecycle());

    let Err(fault) = playbook
        .apply(async { Err(Fault::playbook("reset-orders", "seed data rejected")) })
        .await
    else {
        panic!("the playbook should fault");
    };

    assert_eq!(fault.message, "seed data rejected");
    assert_eq!(playbook.lifecycle().state(), RunnableState::Starting);
    assert_eq!(
        observer.states(),
        vec![RunnableState::NotStarted, RunnableState::Starting]
    );
}

#[tokio::test]
async fn apply_successful_body_holds_the_run_open_until_the_handle_drops() {
    let playbook = ManagedPlaybook::new("reset-orders");
    let observer = observed(playbook.lifecycle());

    let active = playbook
        .apply(async { Ok(probe_active_playbook(playbook.lifecycle())) })
        .await
        .expect("the playbook should run");
    assert_eq!(playbook.lifecycle().state(), RunnableState::Started);

    drop(active);

    assert_eq!(playbook.lifecycle().state(), RunnableState::Stopped);
    assert_eq!(
        observer.states(),
        vec![
            RunnableState::NotStarted,
            RunnableState::Starting,
            RunnableState::Started,
            RunnableState::Stopping,
            RunnableState::Stopped
        ]
    );
}

#[tokio::test]
async fn apply_two_overlapping_runs_stop_once_the_last_handle_drops() {
    let playbook = ManagedPlaybook::new("reset-orders");
    let observer = observed(playbook.lifecycle());

    let first = playbook
        .apply(async { Ok(probe_active_playbook(playbook.lifecycle())) })
        .await
        .expect("the first run should start");
    let second = playbook
        .apply(async { Ok(probe_active_playbook(playbook.lifecycle())) })
        .await
        .expect("the second run should start");

    drop(first);
    assert_eq!(playbook.lifecycle().state(), RunnableState::Started);
    drop(second);

    assert_eq!(playbook.lifecycle().state(), RunnableState::Stopped);
    assert_eq!(
        observer.states(),
        vec![
            RunnableState::NotStarted,
            RunnableState::Starting,
            RunnableState::Started,
            RunnableState::Stopping,
            RunnableState::Stopped
        ]
    );
}

#[tokio::test]
async fn apply_rerun_after_the_handle_dropped_reports_a_second_cycle() {
    let playbook = ManagedPlaybook::new("reset-orders");
    let observer = observed(playbook.lifecycle());

    let first = playbook
        .apply(async { Ok(probe_active_playbook(playbook.lifecycle())) })
        .await
        .expect("the first run should start");
    drop(first);
    let second = playbook
        .apply(async { Ok(probe_active_playbook(playbook.lifecycle())) })
        .await
        .expect("the second run should start");
    drop(second);

    assert_eq!(
        observer.states(),
        vec![
            RunnableState::NotStarted,
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
async fn apply_after_a_fault_clears_the_previous_fault_from_the_next_report() {
    let playbook = ManagedPlaybook::new("reset-orders");
    let observer = observed(playbook.lifecycle());
    playbook
        .lifecycle()
        .report_fault(Fault::playbook("reset-orders", "seed data rejected"));

    let active: Box<dyn ActivePlaybook> = playbook
        .apply(async { Ok(probe_active_playbook(playbook.lifecycle())) })
        .await
        .expect("the playbook should run");

    assert_eq!(active.faults(), Vec::new());
    assert_eq!(observer.faults_for("reset-orders").len(), 1);
}
