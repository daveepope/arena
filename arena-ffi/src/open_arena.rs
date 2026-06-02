use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::closed_arena::{OpenArenaHandle, OpenArenaRuntimeState};
use crate::panic_payload::panic_message;

#[no_mangle]
pub extern "C" fn arena_close(handle: *mut OpenArenaHandle) {
    if handle.is_null() {
        return;
    }
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let runtime_state = unsafe { OpenArenaRuntimeState::from_raw(handle) };
        let mut arena = runtime_state
            .runtime
            .block_on(runtime_state.state.lock());
        let arena = arena.take();
        if let Some(arena) = arena {
            runtime_state.runtime.block_on(arena.close());
        } else {
            tracing::warn!(
                target: "arena::ffi",
                phase = "arena_close_skip",
                "arena close skipped; arena slot empty (stale or duplicate close handle?)"
            );
        }
    }));
    if let Err(payload) = outcome {
        tracing::error!(
            panic_message = %panic_message(&payload),
            op = "arena_close",
            "panic while closing arena"
        );
    }
}
