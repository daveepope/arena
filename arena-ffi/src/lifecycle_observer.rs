use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};

use arc_swap::ArcSwap;
use arena::lifecycle::{ArenaLifecycleObserver, ArenaState};

use crate::arena_state::state_json;
use crate::boundary::call_across_boundary;

pub type ArenaLifecycleCallback =
    unsafe extern "C" fn(state_json_utf8: *const c_char, user_data: *mut c_void);

struct ForwardStateToCAbi {
    token: u64,
    func: ArenaLifecycleCallback,
    binding: *mut c_void,
}

unsafe impl Send for ForwardStateToCAbi {}
unsafe impl Sync for ForwardStateToCAbi {}

impl ArenaLifecycleObserver for ForwardStateToCAbi {
    fn on_state(&self, state: &ArenaState) {
        if !observer_is_registered(self.token) {
            return;
        }
        let Ok(payload) = CString::new(state_json(state)) else {
            return;
        };
        unsafe { (self.func)(payload.as_ptr(), self.binding) };
    }
}

#[derive(Clone)]
struct RegisteredObserver {
    token: u64,
    recipient: Arc<dyn ArenaLifecycleObserver>,
}

static REGISTERED_OBSERVERS: LazyLock<ArcSwap<Vec<RegisteredObserver>>> =
    LazyLock::new(|| ArcSwap::from_pointee(Vec::new()));

static NEXT_OBSERVER_TOKEN: AtomicU64 = AtomicU64::new(1);

fn observer_is_registered(token: u64) -> bool {
    REGISTERED_OBSERVERS
        .load()
        .iter()
        .any(|entry| entry.token == token)
}

pub(crate) fn registered_observers() -> Vec<Arc<dyn ArenaLifecycleObserver>> {
    REGISTERED_OBSERVERS
        .load()
        .iter()
        .map(|entry| Arc::clone(&entry.recipient))
        .collect()
}

#[no_mangle]
pub extern "C" fn arena_add_lifecycle_observer(
    callback: Option<ArenaLifecycleCallback>,
    user_data: *mut c_void,
) -> u64 {
    let Some(callback) = callback else {
        return 0;
    };
    let binding = user_data as usize;
    call_across_boundary(move || {
        let token = NEXT_OBSERVER_TOKEN.fetch_add(1, Ordering::Relaxed);
        let recipient: Arc<dyn ArenaLifecycleObserver> = Arc::new(ForwardStateToCAbi {
            token,
            func: callback,
            binding: binding as *mut c_void,
        });
        REGISTERED_OBSERVERS.rcu(|current| {
            let mut next = (**current).clone();
            next.push(RegisteredObserver {
                token,
                recipient: Arc::clone(&recipient),
            });
            next
        });
        token
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn arena_remove_lifecycle_observer(token: u64) {
    if token == 0 {
        return;
    }
    let _ = call_across_boundary(move || {
        REGISTERED_OBSERVERS.rcu(|current| {
            let next: Vec<RegisteredObserver> = (**current)
                .iter()
                .filter(|entry| entry.token != token)
                .cloned()
                .collect();
            next
        });
    });
}
