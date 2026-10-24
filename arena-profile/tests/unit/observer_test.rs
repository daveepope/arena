use std::sync::{Arc, Mutex};

use arena::lifecycle::RunnableState;
use arena_profile::observer::{
    notify, register_observer, report, report_complete, unregister_observer, CpuProfileEvent,
    CpuProfileObserver,
};

#[derive(Default)]
struct RecordingObserver {
    events: Mutex<Vec<CpuProfileEvent>>,
}

impl CpuProfileObserver for RecordingObserver {
    fn on_cpu_profile(&self, event: &CpuProfileEvent) {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(event.clone());
    }
}

impl RecordingObserver {
    fn events_for(&self, component_id: &str) -> Vec<CpuProfileEvent> {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|event| event.component_id == component_id)
            .cloned()
            .collect()
    }
}

struct PanickingObserver;

impl CpuProfileObserver for PanickingObserver {
    fn on_cpu_profile(&self, _event: &CpuProfileEvent) {
        panic!("observer blew up");
    }
}

struct Registration {
    token: u64,
}

impl Drop for Registration {
    fn drop(&mut self) {
        unregister_observer(self.token);
    }
}

fn register(recipient: Arc<dyn CpuProfileObserver>) -> Registration {
    Registration {
        token: register_observer(recipient),
    }
}

#[test]
fn cpu_profile_event_new_without_folded_carries_no_samples() {
    let event = CpuProfileEvent::new("svc", RunnableState::Starting);

    assert_eq!(event.component_id, "svc");
    assert_eq!(event.state, RunnableState::Starting);
    assert!(event.folded.is_none());
}

#[test]
fn cpu_profile_event_with_folded_carries_the_samples() {
    let event = CpuProfileEvent::new("svc", RunnableState::Stopped).with_folded("main;a 1\n");

    assert_eq!(event.folded.as_deref(), Some("main;a 1\n"));
}

#[test]
fn register_observer_each_registration_returns_a_distinct_token() {
    let first = register(Arc::new(RecordingObserver::default()));
    let second = register(Arc::new(RecordingObserver::default()));

    assert_ne!(first.token, second.token);
}

#[test]
fn notify_registered_observer_receives_the_event() {
    let recipient = Arc::new(RecordingObserver::default());
    let _registration = register(Arc::clone(&recipient) as Arc<dyn CpuProfileObserver>);

    notify(&CpuProfileEvent::new("notified", RunnableState::Started));

    let events = recipient.events_for("notified");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].state, RunnableState::Started);
}

#[test]
fn notify_several_registered_observers_all_receive_the_event() {
    let first = Arc::new(RecordingObserver::default());
    let second = Arc::new(RecordingObserver::default());
    let _first = register(Arc::clone(&first) as Arc<dyn CpuProfileObserver>);
    let _second = register(Arc::clone(&second) as Arc<dyn CpuProfileObserver>);

    notify(&CpuProfileEvent::new("fanned-out", RunnableState::Stopping));

    assert_eq!(first.events_for("fanned-out").len(), 1);
    assert_eq!(second.events_for("fanned-out").len(), 1);
}

#[test]
fn notify_panicking_observer_still_reaches_the_remaining_observers() {
    let recipient = Arc::new(RecordingObserver::default());
    let _panicking = register(Arc::new(PanickingObserver));
    let _recording = register(Arc::clone(&recipient) as Arc<dyn CpuProfileObserver>);

    notify(&CpuProfileEvent::new("survives", RunnableState::Faulted));

    assert_eq!(recipient.events_for("survives").len(), 1);
}

#[test]
fn unregister_observer_stops_delivery_to_that_observer() {
    let recipient = Arc::new(RecordingObserver::default());
    let token = register_observer(Arc::clone(&recipient) as Arc<dyn CpuProfileObserver>);

    unregister_observer(token);
    notify(&CpuProfileEvent::new("unregistered", RunnableState::Started));

    assert!(recipient.events_for("unregistered").is_empty());
}

#[test]
fn unregister_observer_unknown_token_leaves_registrations_intact() {
    let recipient = Arc::new(RecordingObserver::default());
    let _registration = register(Arc::clone(&recipient) as Arc<dyn CpuProfileObserver>);

    unregister_observer(u64::MAX);
    notify(&CpuProfileEvent::new("still-there", RunnableState::Started));

    assert_eq!(recipient.events_for("still-there").len(), 1);
}

#[test]
fn report_each_lifecycle_state_is_delivered_verbatim() {
    let recipient = Arc::new(RecordingObserver::default());
    let _registration = register(Arc::clone(&recipient) as Arc<dyn CpuProfileObserver>);

    let states = [
        RunnableState::Starting,
        RunnableState::Started,
        RunnableState::Stopping,
        RunnableState::Stopped,
        RunnableState::Faulted,
    ];
    for state in states {
        report("transitions", state);
    }

    let delivered: Vec<RunnableState> = recipient
        .events_for("transitions")
        .iter()
        .map(|event| event.state)
        .collect();
    assert_eq!(delivered, states.to_vec());
}

#[test]
fn report_complete_delivers_stopped_with_the_folded_stacks() {
    let recipient = Arc::new(RecordingObserver::default());
    let _registration = register(Arc::clone(&recipient) as Arc<dyn CpuProfileObserver>);

    report_complete("completed", "main;handler;compute 42\n");

    let events = recipient.events_for("completed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].state, RunnableState::Stopped);
    assert_eq!(events[0].folded.as_deref(), Some("main;handler;compute 42\n"));
}
