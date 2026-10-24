use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::fault::panic_message;

pub trait Observer<E>: Send + Sync {
    fn on_event(&self, event: &E);

    fn flush(&self, _deadline: Instant) {}
}

pub trait Publisher<E>: Send + Sync {
    fn publish(&self, event: &E);

    fn flush(&self, _deadline: Instant) {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Registration(u64);

struct Entry<T: ?Sized> {
    registration: Registration,
    recipient: Arc<T>,
}

pub struct EventBus<E> {
    channel: &'static str,
    publishers: Mutex<Vec<Entry<dyn Publisher<E>>>>,
    observers: Mutex<Vec<Entry<dyn Observer<E>>>>,
    next_registration: AtomicU64,
}

impl<E> EventBus<E> {
    pub fn new(channel: &'static str) -> Self {
        Self {
            channel,
            publishers: Mutex::new(Vec::new()),
            observers: Mutex::new(Vec::new()),
            next_registration: AtomicU64::new(1),
        }
    }

    pub fn register_publisher(&self, publisher: Arc<dyn Publisher<E>>) -> Registration {
        let registration = self.next();
        self.publishers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(Entry {
                registration,
                recipient: publisher,
            });
        registration
    }

    pub fn unregister_publisher(&self, registration: Registration) {
        self.publishers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|entry| entry.registration != registration);
    }

    pub fn register_observer(&self, observer: Arc<dyn Observer<E>>) -> Registration {
        let registration = self.next();
        self.observers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(Entry {
                registration,
                recipient: observer,
            });
        registration
    }

    pub fn unregister_observer(&self, registration: Registration) {
        self.observers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|entry| entry.registration != registration);
    }

    pub fn unregister_all_observers(&self) {
        self.observers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    pub fn observer_count(&self) -> usize {
        self.observers.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn publisher_count(&self) -> usize {
        self.publishers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    pub fn publish(&self, event: &E) {
        for observer in self.registered_observers() {
            let outcome = catch_unwind(AssertUnwindSafe(|| observer.on_event(event)));
            if let Err(payload) = outcome {
                tracing::error!(
                    channel = %self.channel,
                    panic_message = %panic_message(payload.as_ref()),
                    phase = "observer_panic",
                    "observer panicked; delivery continues"
                );
            }
        }
    }

    #[doc(hidden)]
    pub fn flush(&self, budget: Duration) {
        let deadline = Instant::now() + budget;
        for publisher in self.registered_publishers() {
            self.flush_one(deadline, || publisher.flush(deadline), "publisher_flush_panic");
        }
        for observer in self.registered_observers() {
            self.flush_one(deadline, || observer.flush(deadline), "observer_flush_panic");
        }
    }

    fn flush_one(&self, deadline: Instant, flush: impl FnOnce(), phase: &'static str) {
        if Instant::now() >= deadline {
            tracing::warn!(
                channel = %self.channel,
                phase = "flush_budget_exhausted",
                "flush budget exhausted; remaining recipients were not drained"
            );
            return;
        }
        if let Err(payload) = catch_unwind(AssertUnwindSafe(flush)) {
            tracing::error!(
                channel = %self.channel,
                panic_message = %panic_message(payload.as_ref()),
                phase = %phase,
                "flush panicked; teardown continues"
            );
        }
    }

    fn registered_observers(&self) -> Vec<Arc<dyn Observer<E>>> {
        self.observers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|entry| Arc::clone(&entry.recipient))
            .collect()
    }

    fn registered_publishers(&self) -> Vec<Arc<dyn Publisher<E>>> {
        self.publishers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|entry| Arc::clone(&entry.recipient))
            .collect()
    }

    fn next(&self) -> Registration {
        Registration(self.next_registration.fetch_add(1, Ordering::Relaxed))
    }
}

impl<E> Publisher<E> for EventBus<E>
where
    E: Send + Sync,
{
    fn publish(&self, event: &E) {
        EventBus::publish(self, event);
    }
}
