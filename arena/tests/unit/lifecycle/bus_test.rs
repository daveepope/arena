use arena::lifecycle::bus::{EventBus, Observer, Publisher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const FLUSH_BUDGET: Duration = Duration::from_secs(5);
const CHANNEL: &str = "test-channel";

#[derive(Clone, Debug, PartialEq, Eq)]
struct Note {
    text: String,
}

impl Note {
    fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }
}

struct RecordingObserver {
    received: Mutex<Vec<Note>>,
    flushes: Mutex<usize>,
}

impl RecordingObserver {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            received: Mutex::new(Vec::new()),
            flushes: Mutex::new(0),
        })
    }

    fn received(&self) -> Vec<Note> {
        self.received.lock().expect("received lock").clone()
    }

    fn flush_count(&self) -> usize {
        *self.flushes.lock().expect("flush lock")
    }
}

impl Observer<Note> for RecordingObserver {
    fn on_event(&self, event: &Note) {
        self.received.lock().expect("received lock").push(event.clone());
    }

    fn flush(&self, _deadline: Instant) {
        *self.flushes.lock().expect("flush lock") += 1;
    }
}

struct PanickingObserver;

impl Observer<Note> for PanickingObserver {
    fn on_event(&self, _event: &Note) {
        panic!("this observer always panics");
    }
}

struct QueueingPublisher {
    bus: Arc<EventBus<Note>>,
    queued: Mutex<Vec<Note>>,
}

impl QueueingPublisher {
    fn new(bus: Arc<EventBus<Note>>) -> Arc<Self> {
        Arc::new(Self {
            bus,
            queued: Mutex::new(Vec::new()),
        })
    }

    fn queue(&self, note: Note) {
        self.queued.lock().expect("queued lock").push(note);
    }
}

impl Publisher<Note> for QueueingPublisher {
    fn publish(&self, event: &Note) {
        self.bus.publish(event);
    }

    fn flush(&self, _deadline: Instant) {
        for note in self.queued.lock().expect("queued lock").drain(..) {
            self.bus.publish(&note);
        }
    }
}

fn bus() -> Arc<EventBus<Note>> {
    Arc::new(EventBus::new(CHANNEL))
}

#[test]
fn publish_registered_observer_receives_the_event() {
    let bus = bus();
    let observer = RecordingObserver::new();
    bus.register_observer(observer.clone());
    let publisher = QueueingPublisher::new(bus.clone());
    bus.register_publisher(publisher.clone());

    publisher.publish(&Note::new("opened"));

    assert_eq!(observer.received(), vec![Note::new("opened")]);
}

#[test]
fn publish_unregistered_observer_receives_nothing_further() {
    let bus = bus();
    let observer = RecordingObserver::new();
    let registration = bus.register_observer(observer.clone());
    let publisher = QueueingPublisher::new(bus.clone());

    publisher.publish(&Note::new("before"));
    bus.unregister_observer(registration);
    publisher.publish(&Note::new("after"));

    assert_eq!(observer.received(), vec![Note::new("before")]);
}

#[test]
fn publish_two_observers_both_receive_the_event() {
    let bus = bus();
    let first = RecordingObserver::new();
    let second = RecordingObserver::new();
    bus.register_observer(first.clone());
    bus.register_observer(second.clone());

    bus.publish(&Note::new("broadcast"));

    assert_eq!(first.received(), vec![Note::new("broadcast")]);
    assert_eq!(second.received(), vec![Note::new("broadcast")]);
}

#[test]
fn publish_panicking_observer_does_not_stop_the_others() {
    let bus = bus();
    bus.register_observer(Arc::new(PanickingObserver));
    let surviving = RecordingObserver::new();
    bus.register_observer(surviving.clone());

    bus.publish(&Note::new("delivered anyway"));

    assert_eq!(surviving.received(), vec![Note::new("delivered anyway")]);
}

#[test]
fn flush_queued_publisher_reaches_the_observer() {
    let bus = bus();
    let observer = RecordingObserver::new();
    bus.register_observer(observer.clone());
    let publisher = QueueingPublisher::new(bus.clone());
    bus.register_publisher(publisher.clone());
    publisher.queue(Note::new("queued"));

    assert_eq!(observer.received(), Vec::new());

    bus.flush(FLUSH_BUDGET);

    assert_eq!(observer.received(), vec![Note::new("queued")]);
}

#[test]
fn flush_unregistered_publisher_is_not_drained() {
    let bus = bus();
    let observer = RecordingObserver::new();
    bus.register_observer(observer.clone());
    let publisher = QueueingPublisher::new(bus.clone());
    let registration = bus.register_publisher(publisher.clone());
    publisher.queue(Note::new("abandoned"));

    bus.unregister_publisher(registration);
    bus.flush(FLUSH_BUDGET);

    assert_eq!(observer.received(), Vec::new());
}

#[test]
fn flush_exhausted_budget_leaves_recipients_undrained() {
    let bus = bus();
    let observer = RecordingObserver::new();
    bus.register_observer(observer.clone());
    let publisher = QueueingPublisher::new(bus.clone());
    bus.register_publisher(publisher.clone());
    publisher.queue(Note::new("too late"));

    bus.flush(Duration::ZERO);

    assert_eq!(observer.received(), Vec::new());
    assert_eq!(observer.flush_count(), 0);
}

#[test]
fn flush_called_twice_drains_once() {
    let bus = bus();
    let observer = RecordingObserver::new();
    bus.register_observer(observer.clone());
    let publisher = QueueingPublisher::new(bus.clone());
    bus.register_publisher(publisher.clone());
    publisher.queue(Note::new("once"));

    bus.flush(FLUSH_BUDGET);
    bus.flush(FLUSH_BUDGET);

    assert_eq!(observer.received(), vec![Note::new("once")]);
}

#[test]
fn register_and_unregister_are_reflected_in_the_counts() {
    let bus = bus();
    let observer = RecordingObserver::new();
    let publisher = QueueingPublisher::new(bus.clone());

    let observer_registration = bus.register_observer(observer);
    let publisher_registration = bus.register_publisher(publisher);
    assert_eq!(bus.observer_count(), 1);
    assert_eq!(bus.publisher_count(), 1);

    bus.unregister_observer(observer_registration);
    bus.unregister_publisher(publisher_registration);

    assert_eq!(bus.observer_count(), 0);
    assert_eq!(bus.publisher_count(), 0);
}

#[test]
fn unregister_removes_only_the_named_registration() {
    let bus = bus();
    let kept = RecordingObserver::new();
    let dropped = RecordingObserver::new();
    bus.register_observer(kept.clone());
    let dropped_registration = bus.register_observer(dropped.clone());

    bus.unregister_observer(dropped_registration);
    bus.publish(&Note::new("for the survivor"));

    assert_eq!(kept.received(), vec![Note::new("for the survivor")]);
    assert_eq!(dropped.received(), Vec::new());
}
