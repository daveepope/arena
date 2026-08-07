use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::closed_arena::{OpenArenaHandle, OpenArenaRuntimeState};
use crate::logging;
use crate::panic_payload::panic_message;

#[no_mangle]
pub extern "C" fn arena_close(handle: *mut OpenArenaHandle) {
    if handle.is_null() {
        return;
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let runtime_state = unsafe { OpenArenaRuntimeState::from_raw(handle) };
        runtime_state.runtime.block_on(async {
            let arena = runtime_state.state.lock().await.take();
            if let Some(arena) = arena {
                arena.close().await;
            } else {
                tracing::warn!(
                    target: "arena::ffi",
                    phase = "arena_close_skip",
                    "arena close skipped; arena slot empty (stale or duplicate close handle?)"
                );
            }
        });
    }));
    logging::dispatcher_allowlists_reset();
    if let Err(payload) = outcome {
        tracing::error!(
            panic_message = %panic_message(&payload),
            op = "arena_close",
            "panic while closing arena"
        );
    }
}
